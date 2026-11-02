// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2023 Takashi Sakamoto

use {
    super::{ff802_model::*, ucx_model::*, *},
    alsactl::{ElemEventMask, ElemId, ElemValue},
    std::sync::mpsc,
};

pub type Ff802Runtime = FfLatterRuntime<Ff802Model>;
pub type FfUcxRuntime = FfLatterRuntime<UcxModel>;

enum Event {
    Shutdown,
    Disconnected,
    BusReset(u32),
    Elem(ElemId, ElemEventMask),
    Timer,
    DspReport,
}

/// Model which reads parameter reports from the DSP, so that changes made at the hardware (e.g.
/// the gain knob) reach the elements.
pub trait DspReportModel<O> {
    /// Whether to poll the reports periodically.
    fn dsp_report_available(&self) -> bool {
        false
    }

    /// Poll the reports, update the cached state, and add the elements to refresh.
    fn poll_dsp_reports(&mut self, _: &mut O, _: &mut Vec<ElemId>) -> Result<(), Error> {
        Ok(())
    }
}

pub struct FfLatterRuntime<T>
where
    T: Default
        + CtlModel<(SndFireface, FwNode)>
        + MeasureModel<(SndFireface, FwNode)>
        + DspReportModel<(SndFireface, FwNode)>,
{
    unit: (SndFireface, FwNode),
    model: T,
    card_cntr: CardCntr,
    rx: mpsc::Receiver<Event>,
    tx: mpsc::SyncSender<Event>,
    dispatchers: Vec<Dispatcher>,
    timer: Option<Dispatcher>,
    measured_elem_id_list: Vec<ElemId>,
    reported_elem_id_list: Vec<ElemId>,
}

impl<T> FfLatterRuntime<T>
where
    T: Default
        + CtlModel<(SndFireface, FwNode)>
        + MeasureModel<(SndFireface, FwNode)>
        + DspReportModel<(SndFireface, FwNode)>,
{
    pub(crate) fn new(unit: SndFireface, node: FwNode, card_cntr: CardCntr) -> Result<Self, Error> {
        // Use uni-directional channel for communication to child threads.
        let (tx, rx) = mpsc::sync_channel(32);

        let runtime = FfLatterRuntime {
            unit: (unit, node),
            model: Default::default(),
            card_cntr,
            rx,
            tx,
            dispatchers: Default::default(),
            timer: None,
            measured_elem_id_list: Default::default(),
            reported_elem_id_list: Default::default(),
        };

        Ok(runtime)
    }

    pub(crate) fn listen(&mut self) -> Result<(), Error> {
        self.launch_node_event_dispatcher()?;
        self.launch_system_event_dispatcher()?;

        let enter = debug_span!("cache").entered();
        self.model.cache(&mut self.unit)?;
        enter.exit();

        let enter = debug_span!("load").entered();
        self.model.load(&mut self.card_cntr)?;

        self.model
            .get_measure_elem_list(&mut self.measured_elem_id_list);
        if self.measured_elem_id_list.len() > 0 {
            let elem_id = ElemId::new_by_name(ElemIfaceType::Mixer, 0, 0, TIMER_NAME, 0);
            let _ = self.card_cntr.add_bool_elems(&elem_id, 1, 1, true)?;
        }

        enter.exit();

        if self.model.dsp_report_available() {
            self.launch_dsp_report_dispatcher()?;
        }

        Ok(())
    }

    pub(crate) fn run(&mut self) -> Result<(), Error> {
        let enter = debug_span!("event").entered();
        loop {
            if let Ok(ev) = self.rx.recv() {
                match ev {
                    Event::Shutdown => break,
                    Event::Disconnected => break,
                    Event::BusReset(generation) => {
                        debug!("IEEE 1394 bus is updated: {}", generation);
                    }
                    Event::Elem(elem_id, events) => {
                        let _enter = debug_span!("element").entered();

                        debug!(
                            numid = elem_id.numid(),
                            name = elem_id.name().as_str(),
                            iface = ?elem_id.iface(),
                            device_id = elem_id.device_id(),
                            subdevice_id = elem_id.subdevice_id(),
                            index = elem_id.index(),
                        );

                        if elem_id.name() != TIMER_NAME {
                            let _ = self.card_cntr.dispatch_elem_event(
                                &mut self.unit,
                                &elem_id,
                                &events,
                                &mut self.model,
                            );
                        } else {
                            let mut elem_value = ElemValue::new();
                            let _ = self
                                .card_cntr
                                .card
                                .read_elem_value(&elem_id, &mut elem_value)
                                .map(|_| {
                                    let val = elem_value.boolean()[0];
                                    if val {
                                        let _ = self.start_interval_timer();
                                    } else {
                                        self.stop_interval_timer();
                                    }
                                });
                        }
                    }
                    Event::DspReport => {
                        let _enter = debug_span!("dsp-report").entered();
                        self.reported_elem_id_list.clear();
                        let res = self
                            .model
                            .poll_dsp_reports(&mut self.unit, &mut self.reported_elem_id_list);
                        if let Err(e) = res {
                            debug!("poll of DSP reports failed: {}", e);
                        }
                        if !self.reported_elem_id_list.is_empty() {
                            let _ = self.card_cntr.refresh_elems::<(SndFireface, FwNode), _>(
                                &self.reported_elem_id_list,
                                &mut self.model,
                            );
                        }
                    }
                    Event::Timer => {
                        let _enter = debug_span!("timer").entered();
                        let _ = self.card_cntr.measure_elems(
                            &mut self.unit,
                            &self.measured_elem_id_list,
                            &mut self.model,
                        );
                    }
                }
            }
        }

        enter.exit();

        Ok(())
    }
}

impl<T> Drop for FfLatterRuntime<T>
where
    T: Default
        + CtlModel<(SndFireface, FwNode)>
        + MeasureModel<(SndFireface, FwNode)>
        + DspReportModel<(SndFireface, FwNode)>,
{
    fn drop(&mut self) {
        // At first, stop event loop in all of dispatchers to avoid queueing new events.
        for dispatcher in &mut self.dispatchers {
            dispatcher.stop();
        }

        // Next, consume all events in queue to release blocked thread for sender.
        for _ in self.rx.try_iter() {}

        // Finally Finish I/O threads.
        self.dispatchers.clear();
    }
}

impl<T> FfLatterRuntime<T>
where
    T: Default
        + CtlModel<(SndFireface, FwNode)>
        + MeasureModel<(SndFireface, FwNode)>
        + DspReportModel<(SndFireface, FwNode)>,
{
    fn launch_node_event_dispatcher(&mut self) -> Result<(), Error> {
        let name = NODE_DISPATCHER_NAME.to_string();
        let mut dispatcher = Dispatcher::run(name)?;

        let tx = self.tx.clone();
        dispatcher.attach_alsa_firewire(&self.unit.0, move |_| {
            let _ = tx.send(Event::Disconnected);
        })?;

        let tx = self.tx.clone();
        dispatcher.attach_fw_node(&self.unit.1, move |_| {
            let _ = tx.send(Event::Disconnected);
        })?;

        let tx = self.tx.clone();
        self.unit.1.connect_bus_update(move |node| {
            let _ = tx.send(Event::BusReset(node.generation()));
        });

        self.dispatchers.push(dispatcher);

        Ok(())
    }

    fn launch_system_event_dispatcher(&mut self) -> Result<(), Error> {
        let name = SYSTEM_DISPATCHER_NAME.to_string();
        let mut dispatcher = Dispatcher::run(name)?;

        let tx = self.tx.clone();
        dispatcher.attach_signal_handler(signal::Signal::SIGINT, move || {
            let _ = tx.send(Event::Shutdown);
            glib::ControlFlow::Break
        });

        let tx = self.tx.clone();
        dispatcher.attach_snd_card(&self.card_cntr.card, |_| {})?;
        self.card_cntr
            .card
            .connect_handle_elem_event(move |_, elem_id, events| {
                let elem_id: ElemId = elem_id.clone();
                let _ = tx.send(Event::Elem(elem_id, events));
            });

        self.dispatchers.push(dispatcher);

        Ok(())
    }

    fn launch_dsp_report_dispatcher(&mut self) -> Result<(), Error> {
        let mut dispatcher = Dispatcher::run(DSP_REPORT_DISPATCHER_NAME.to_string())?;
        let tx = self.tx.clone();
        dispatcher.attach_interval_handler(DSP_REPORT_INTERVAL, move || {
            // Skip the tick when the queue is full, rather than block.
            let _ = tx.try_send(Event::DspReport);
            glib::ControlFlow::Continue
        });
        self.dispatchers.push(dispatcher);
        Ok(())
    }

    fn start_interval_timer(&mut self) -> Result<(), Error> {
        let mut dispatcher = Dispatcher::run(TIMER_DISPATCHER_NAME.to_string())?;
        let tx = self.tx.clone();
        dispatcher.attach_interval_handler(TIMER_INTERVAL, move || {
            let _ = tx.send(Event::Timer);
            glib::ControlFlow::Continue
        });

        self.timer = Some(dispatcher);

        Ok(())
    }

    fn stop_interval_timer(&mut self) {
        self.timer = None;
    }
}
