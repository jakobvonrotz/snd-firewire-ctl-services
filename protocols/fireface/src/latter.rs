// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (c) 2021 Takashi Sakamoto

//! Protocol defined by RME GmbH for latter models of Fireface series.

pub mod ff802;
pub mod ucx;

use super::*;

const CFG_OFFSET: u64 = 0xffff00000014;
const DSP_OFFSET: u64 = 0xffff0000001c;
const METER_OFFSET: u64 = 0xffffff000000;

// For configuration register (0x'ffff'0000'0014).
const CFG_MIDI_TX_LOW_OFFSET_MASK: u32 = 0x0001e000;
const CFG_MIDI_TX_LOW_OFFSET_0180_FLAG: u32 = 0x00010000;
const CFG_MIDI_TX_LOW_OFFSET_0100_FLAG: u32 = 0x00008000;
const CFG_MIDI_TX_LOW_OFFSET_0080_FLAG: u32 = 0x00004000;
const CFG_MIDI_TX_LOW_OFFSET_0000_FLAG: u32 = 0x00002000;

/// Low offset of destination address for MIDI messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FfLatterMidiTxLowOffset {
    /// Between 0x0000 to 0x007c.
    A0000,
    /// Between 0x0080 to 0x00fc.
    A0080,
    /// Between 0x0100 to 0x017c.
    A0100,
    /// Between 0x0180 to 0x01fc.
    A0180,
}

impl Default for FfLatterMidiTxLowOffset {
    fn default() -> Self {
        Self::A0000
    }
}

fn serialize_midi_tx_low_offset(offset: &FfLatterMidiTxLowOffset, quad: &mut u32) {
    *quad &= !CFG_MIDI_TX_LOW_OFFSET_MASK;
    *quad |= match offset {
        FfLatterMidiTxLowOffset::A0000 => CFG_MIDI_TX_LOW_OFFSET_0000_FLAG,
        FfLatterMidiTxLowOffset::A0080 => CFG_MIDI_TX_LOW_OFFSET_0080_FLAG,
        FfLatterMidiTxLowOffset::A0100 => CFG_MIDI_TX_LOW_OFFSET_0100_FLAG,
        FfLatterMidiTxLowOffset::A0180 => CFG_MIDI_TX_LOW_OFFSET_0180_FLAG,
    };
}

fn deserialize_midi_tx_low_offset(offset: &mut FfLatterMidiTxLowOffset, quad: &u32) {
    *offset = match *quad & CFG_MIDI_TX_LOW_OFFSET_MASK {
        CFG_MIDI_TX_LOW_OFFSET_0180_FLAG => FfLatterMidiTxLowOffset::A0180,
        CFG_MIDI_TX_LOW_OFFSET_0100_FLAG => FfLatterMidiTxLowOffset::A0100,
        CFG_MIDI_TX_LOW_OFFSET_0080_FLAG => FfLatterMidiTxLowOffset::A0080,
        CFG_MIDI_TX_LOW_OFFSET_0000_FLAG => FfLatterMidiTxLowOffset::A0000,
        _ => unreachable!(),
    }
}

const LATTER_CONFIG_SIZE: usize = 4;

fn write_config<T: RmeFfOffsetParamsSerialize<U>, U>(
    req: &mut FwReq,
    node: &mut FwNode,
    config: &U,
    timeout_ms: u32,
) -> Result<(), Error> {
    let mut raw = T::serialize_offsets(config);

    req.transaction(
        node,
        FwTcode::WriteQuadletRequest,
        CFG_OFFSET,
        raw.len(),
        &mut raw,
        timeout_ms,
    )
}

// For status register (0x'ffff'0000'001c).
const STATUS_CLK_RATE_32000: u32 = 0x00;
const STATUS_CLK_RATE_44100: u32 = 0x01;
const STATUS_CLK_RATE_48000: u32 = 0x02;
const STATUS_CLK_RATE_64000: u32 = 0x04;
const STATUS_CLK_RATE_88200: u32 = 0x05;
const STATUS_CLK_RATE_96000: u32 = 0x06;
const STATUS_CLK_RATE_128000: u32 = 0x08;
const STATUS_CLK_RATE_176400: u32 = 0x09;
const STATUS_CLK_RATE_192000: u32 = 0x0a;
const STATUS_CLK_RATE_NONE: u32 = 0x0f;

fn serialize_clock_rate(clock_rate: &ClkNominalRate, quad: &mut u32, shift: usize) {
    let val = match clock_rate {
        ClkNominalRate::R32000 => STATUS_CLK_RATE_32000,
        ClkNominalRate::R44100 => STATUS_CLK_RATE_44100,
        ClkNominalRate::R48000 => STATUS_CLK_RATE_48000,
        ClkNominalRate::R64000 => STATUS_CLK_RATE_64000,
        ClkNominalRate::R88200 => STATUS_CLK_RATE_88200,
        ClkNominalRate::R96000 => STATUS_CLK_RATE_96000,
        ClkNominalRate::R128000 => STATUS_CLK_RATE_128000,
        ClkNominalRate::R176400 => STATUS_CLK_RATE_176400,
        ClkNominalRate::R192000 => STATUS_CLK_RATE_192000,
    };
    *quad |= val << shift;
}

fn deserialize_clock_rate(clock_rate: &mut ClkNominalRate, quad: &u32, shift: usize) {
    *clock_rate = match (*quad >> shift) & 0x0000000f {
        STATUS_CLK_RATE_32000 => ClkNominalRate::R32000,
        STATUS_CLK_RATE_44100 => ClkNominalRate::R44100,
        STATUS_CLK_RATE_48000 => ClkNominalRate::R48000,
        STATUS_CLK_RATE_64000 => ClkNominalRate::R64000,
        STATUS_CLK_RATE_88200 => ClkNominalRate::R88200,
        STATUS_CLK_RATE_96000 => ClkNominalRate::R96000,
        STATUS_CLK_RATE_128000 => ClkNominalRate::R128000,
        STATUS_CLK_RATE_176400 => ClkNominalRate::R176400,
        STATUS_CLK_RATE_192000 => ClkNominalRate::R192000,
        _ => unreachable!(),
    };
}

fn serialize_clock_rate_optional(
    clock_rate: &Option<ClkNominalRate>,
    quad: &mut u32,
    shift: usize,
) {
    if let Some(r) = clock_rate {
        serialize_clock_rate(r, quad, shift)
    } else {
        *quad |= STATUS_CLK_RATE_NONE << shift;
    }
}

fn deserialize_clock_rate_optional(
    clock_rate: &mut Option<ClkNominalRate>,
    quad: &u32,
    shift: usize,
) {
    if (*quad >> shift) & 0x0000000f != STATUS_CLK_RATE_NONE {
        let mut r = ClkNominalRate::default();
        deserialize_clock_rate(&mut r, quad, shift);
        *clock_rate = Some(r);
    } else {
        *clock_rate = None;
    };
}

const LATTER_STATUS_SIZE: usize = 4;

fn read_status<T: RmeFfOffsetParamsDeserialize<U>, U>(
    req: &mut FwReq,
    node: &mut FwNode,
    status: &mut U,
    timeout_ms: u32,
) -> Result<(), Error> {
    let mut raw = [0; 4];
    req.transaction(
        node,
        FwTcode::ReadQuadletRequest,
        DSP_OFFSET as u64,
        raw.len(),
        &mut raw,
        timeout_ms,
    )
    .map(|_| T::deserialize_offsets(status, &raw))
}

/// Pair each physical input value with its channel number. Line and microphone inputs are placed
/// per model; digital inputs follow both of them.
fn phys_input_channels<'a, T, S>(
    line: &'a [S],
    mic: &'a [S],
    digital: impl Iterator<Item = &'a S> + 'a,
) -> impl Iterator<Item = (usize, S)> + 'a
where
    T: RmeFfLatterSpecification,
    S: Copy + 'a,
{
    let digital_offset = T::LINE_INPUT_COUNT + T::MIC_INPUT_COUNT;
    line.iter()
        .enumerate()
        .map(|(i, &v)| (T::LINE_INPUT_OFFSET + i, v))
        .chain(mic.iter().enumerate().map(|(i, &v)| (T::MIC_INPUT_OFFSET + i, v)))
        .chain(digital.enumerate().map(move |(i, &v)| (digital_offset + i, v)))
}

/// The specification of latter model.
pub trait RmeFfLatterSpecification {
    /// The number of line inputs.
    const LINE_INPUT_COUNT: usize;
    /// The number of microphone inputs.
    const MIC_INPUT_COUNT: usize;
    /// The first physical channel of line inputs. The 802 has line inputs first, while the UCX
    /// has its microphone preamps at channel 0-1.
    const LINE_INPUT_OFFSET: usize = 0;
    /// The first physical channel of microphone inputs.
    const MIC_INPUT_OFFSET: usize = Self::LINE_INPUT_COUNT;
    /// The number of S/PDIF inputs.
    const SPDIF_INPUT_COUNT: usize;
    /// The number of ADAT inputs.
    const ADAT_INPUT_COUNT: usize;
    /// The number of stream inputs.
    const STREAM_INPUT_COUNT: usize;
    /// The number of fx returns.
    const FX_RETURN_COUNT: usize;

    /// The number of line outputs.
    const LINE_OUTPUT_COUNT: usize;
    /// The number of headphone outputs.
    const HP_OUTPUT_COUNT: usize;
    /// The number of S/PDIF outputs.
    const SPDIF_OUTPUT_COUNT: usize;
    /// The number of ADAT outputs.
    const ADAT_OUTPUT_COUNT: usize;

    /// The size of each chunk of meter data in bytes. The last quadlet is the tag.
    const METER_CHUNK_SIZE: usize = 392;
    /// The offset of quadlets for meter levels in each chunk.
    const METER_LEVEL_OFFSET: usize = 256;
}

/// State of meters.
///
/// Each value is between 0x'0000'0000'0000'0000 and 0x'3fff'ffff'ffff'ffff. 0x'0000'0000'0000'001f
/// represents negative infinite.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct FfLatterMeterState {
    /// The number of line inputs.
    pub line_inputs: Vec<i32>,
    /// The number of microphone inputs.
    pub mic_inputs: Vec<i32>,
    /// The number of S/PDIF inputs.
    pub spdif_inputs: Vec<i32>,
    /// The number of ADAT inputs.
    pub adat_inputs: Vec<i32>,
    /// The number of stream inputs.
    pub stream_inputs: Vec<i32>,
    /// The number of line outputs.
    pub line_outputs: Vec<i32>,
    /// The number of headphone outputs.
    pub hp_outputs: Vec<i32>,
    /// The number of S/PDIF outputs.
    pub spdif_outputs: Vec<i32>,
    /// The number of ADAT outputs.
    pub adat_outputs: Vec<i32>,
}

const METER_CHUNK_COUNT: usize = 5;
const METER32_MASK: i32 = 0x07fffff0;

/// The specification of hardware meter.
pub trait RmeFfLatterMeterSpecification: RmeFfLatterSpecification {
    const LEVEL_MIN: i32 = 0x0;
    const LEVEL_MAX: i32 = 0x07fffff0;
    const LEVEL_STEP: i32 = 0x10;

    fn create_meter_state() -> FfLatterMeterState {
        FfLatterMeterState {
            line_inputs: vec![Default::default(); Self::LINE_INPUT_COUNT],
            mic_inputs: vec![Default::default(); Self::MIC_INPUT_COUNT],
            spdif_inputs: vec![Default::default(); Self::SPDIF_INPUT_COUNT],
            adat_inputs: vec![Default::default(); Self::ADAT_INPUT_COUNT],
            stream_inputs: vec![Default::default(); Self::STREAM_INPUT_COUNT],
            line_outputs: vec![Default::default(); Self::LINE_OUTPUT_COUNT],
            hp_outputs: vec![Default::default(); Self::HP_OUTPUT_COUNT],
            spdif_outputs: vec![Default::default(); Self::SPDIF_OUTPUT_COUNT],
            adat_outputs: vec![Default::default(); Self::ADAT_OUTPUT_COUNT],
        }
    }
}

impl<O: RmeFfLatterSpecification> RmeFfLatterMeterSpecification for O {}

// Read data retrieved by each block read transaction consists of below chunks in the order:
//  32 octlets for meters detected by DSP.
//  32 quadlets for meters detected by DSP.
//  2 quadlets for unknown meters.
//  2 quadlets for tags.
//
// The first tag represents the set of content:
//  0x11111111 - hardware outputs
//  0x22222222 - channel strip for hardware inputs
//  0x33333333 - stream inputs
//  0x55555555 - fx bus
//  0x66666666 - hardware inputs
//
//  The maximum value for quadlet is 0x07fffff0. The byte in LSB is 0xf at satulated.
impl<O: RmeFfLatterMeterSpecification> RmeFfOffsetParamsDeserialize<FfLatterMeterState> for O {
    fn deserialize_offsets(state: &mut FfLatterMeterState, raw: &[u8]) {
        assert_eq!(raw.len(), Self::METER_CHUNK_SIZE);

        let mut quadlet = [0; 4];
        quadlet.copy_from_slice(&raw[(Self::METER_CHUNK_SIZE - 4)..]);
        let target = u32::from_le_bytes(quadlet);

        match target {
            // For phys outputs.
            0x11111111 => {
                [
                    (state.line_outputs.iter_mut(), 0),
                    (state.hp_outputs.iter_mut(), Self::LINE_OUTPUT_COUNT),
                    (
                        state.spdif_outputs.iter_mut(),
                        Self::LINE_OUTPUT_COUNT + Self::HP_OUTPUT_COUNT,
                    ),
                    (
                        state.adat_outputs.iter_mut(),
                        Self::LINE_OUTPUT_COUNT + Self::HP_OUTPUT_COUNT + Self::SPDIF_OUTPUT_COUNT,
                    ),
                ]
                .iter_mut()
                .for_each(|(iter, offset)| {
                    iter.enumerate().for_each(|(i, meter)| {
                        let pos = Self::METER_LEVEL_OFFSET + (*offset + i) * 4;
                        quadlet.copy_from_slice(&raw[pos..(pos + 4)]);
                        *meter = i32::from_le_bytes(quadlet) & METER32_MASK;
                    });
                });
            }
            // For stream inputs.
            0x33333333 => {
                state
                    .stream_inputs
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, meter)| {
                        let pos = Self::METER_LEVEL_OFFSET + i * 4;
                        quadlet.copy_from_slice(&raw[pos..(pos + 4)]);
                        *meter = i32::from_le_bytes(quadlet) & METER32_MASK;
                    });
            }
            // For phys inputs.
            0x66666666 => {
                [
                    (state.line_inputs.iter_mut(), Self::LINE_INPUT_OFFSET),
                    (state.mic_inputs.iter_mut(), Self::MIC_INPUT_OFFSET),
                    (
                        state.spdif_inputs.iter_mut(),
                        Self::LINE_INPUT_COUNT + Self::MIC_INPUT_COUNT,
                    ),
                    (
                        state.adat_inputs.iter_mut(),
                        Self::LINE_INPUT_COUNT + Self::MIC_INPUT_COUNT + Self::SPDIF_INPUT_COUNT,
                    ),
                ]
                .iter_mut()
                .for_each(|(iter, offset)| {
                    iter.enumerate().for_each(|(i, meter)| {
                        let pos = Self::METER_LEVEL_OFFSET + (*offset + i) * 4;
                        quadlet.copy_from_slice(&raw[pos..(pos + 4)]);
                        *meter = i32::from_le_bytes(quadlet) & METER32_MASK;
                    });
                });
            }
            _ => (),
        }
    }
}

impl<O> RmeFfCacheableParamsOperation<FfLatterMeterState> for O
where
    O: RmeFfOffsetParamsDeserialize<FfLatterMeterState> + RmeFfLatterSpecification,
{
    fn cache_wholly(
        req: &mut FwReq,
        node: &mut FwNode,
        state: &mut FfLatterMeterState,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        (0..METER_CHUNK_COUNT).try_for_each(|_| {
            let mut raw = vec![0; Self::METER_CHUNK_SIZE];
            req.transaction(
                node,
                FwTcode::ReadBlockRequest,
                METER_OFFSET as u64,
                raw.len(),
                &mut raw,
                timeout_ms,
            )
            .map(|_| Self::deserialize_offsets(state, &raw))
        })
    }
}

const VIRT_PORT_CMD_FLAG: u32 = 0x40000000;
const ODD_PARITY_FLAG: u32 = 0x80000000;

fn create_phys_port_cmd(ch: u8, cmd: u8, coef: i16) -> u32 {
    ((ch as u32) << 24) | ((cmd as u32) << 16) | (u16::from_le_bytes(coef.to_le_bytes()) as u32)
}

fn create_virt_port_cmd(mixer_step: u16, mixer: u16, ch: u16, coef: u16) -> u32 {
    VIRT_PORT_CMD_FLAG | (((mixer_step * mixer + ch) as u32) << 16) | (coef as u32)
}

fn write_dsp_cmd(
    req: &mut FwReq,
    node: &mut FwNode,
    mut cmd: u32,
    timeout_ms: u32,
) -> Result<(), Error> {
    // Add odd parity.
    if (0..32).fold(0x01, |count, shift| count ^ (cmd >> shift) & 0x1) > 0 {
        cmd |= ODD_PARITY_FLAG;
    }
    let mut raw = cmd.to_le_bytes();
    req.transaction(
        node,
        FwTcode::WriteQuadletRequest,
        DSP_OFFSET as u64,
        raw.len(),
        &mut raw,
        timeout_ms,
    )
}

fn write_dsp_cmds(
    req: &mut FwReq,
    node: &mut FwNode,
    curr: &[u32],
    cmds: &[u32],
    timeout_ms: u32,
) -> Result<(), Error> {
    cmds.iter()
        .zip(curr)
        .filter(|(n, o)| !n.eq(o))
        .try_for_each(|(&cmd, _)| write_dsp_cmd(req, node, cmd, timeout_ms))
}

/// Serialize commands for parameters.
pub trait RmeFfCommandParamsSerialize<T> {
    /// Serialize parameters into commands.
    fn serialize_commands(params: &T) -> Vec<u32>;
}

/// Deserialize commands for parameters.
pub trait RmeFfCommandParamsDeserialize<T> {
    /// Derialize parameters from commands.
    fn deserialize_commands(params: &mut T, raw: &[u32]);
}

/// Operation for parameters which can be updated wholly at once.
pub trait RmeFfWhollyCommandableParamsOperation<T> {
    /// Update registers for whole parameters.
    fn command_wholly(
        req: &mut FwReq,
        node: &mut FwNode,
        params: &T,
        timeout_ms: u32,
    ) -> Result<(), Error>;
}

impl<O, T> RmeFfWhollyCommandableParamsOperation<T> for O
where
    O: RmeFfCommandParamsSerialize<T>,
{
    fn command_wholly(
        req: &mut FwReq,
        node: &mut FwNode,
        params: &T,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        let cmds = Self::serialize_commands(params);
        cmds.iter()
            .try_for_each(|&cmd| write_dsp_cmd(req, node, cmd, timeout_ms))
    }
}

/// Operation for parameters which can be updated partially.
pub trait RmeFfPartiallyCommandableParamsOperation<T> {
    fn command_partially(
        req: &mut FwReq,
        node: &mut FwNode,
        params: &mut T,
        update: T,
        timeout_ms: u32,
    ) -> Result<(), Error>;
}

impl<O, T> RmeFfPartiallyCommandableParamsOperation<T> for O
where
    O: RmeFfCommandParamsSerialize<T>,
{
    fn command_partially(
        req: &mut FwReq,
        node: &mut FwNode,
        params: &mut T,
        update: T,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        let old = Self::serialize_commands(params);
        let new = Self::serialize_commands(&update);

        write_dsp_cmds(req, node, &old, &new, timeout_ms).map(|_| *params = update)
    }
}

/// The specification of DSP.
///
/// DSP is configurable by quadlet write request with command aligned to little endian, which
/// consists of two parts; 16 bit target and 16 bit coefficient. The command has odd parity
/// bit in its most significant bit against the rest of bits.
pub trait RmeFfLatterDspSpecification: RmeFfLatterSpecification {
    const PHYS_INPUT_COUNT: usize = Self::LINE_INPUT_COUNT
        + Self::MIC_INPUT_COUNT
        + Self::SPDIF_INPUT_COUNT
        + Self::ADAT_INPUT_COUNT;
    const INPUT_COUNT: usize = Self::PHYS_INPUT_COUNT + Self::STREAM_INPUT_COUNT;
    const OUTPUT_COUNT: usize = Self::LINE_OUTPUT_COUNT
        + Self::HP_OUTPUT_COUNT
        + Self::SPDIF_OUTPUT_COUNT
        + Self::ADAT_OUTPUT_COUNT;

    const STREAM_OFFSET: u16 = 0x0020;
    const MIXER_STEP: u16 = Self::STREAM_OFFSET * 2;
}

impl<O: RmeFfLatterSpecification> RmeFfLatterDspSpecification for O {}

const INPUT_TO_FX_CMD: u8 = 0x01;
const INPUT_STEREO_LINK_CMD: u8 = 0x02;
const INPUT_INVERT_PHASE_CMD: u8 = 0x06;
const INPUT_LINE_GAIN_CMD: u8 = 0x07;
const INPUT_LINE_LEVEL_CMD: u8 = 0x08;
const INPUT_MIC_POWER_CMD: u8 = 0x08;
const INPUT_MIC_INST_CMD: u8 = 0x09;

const OUTPUT_VOL_CMD: u8 = 0x00;
const OUTPUT_STEREO_BALANCE_CMD: u8 = 0x01;
const OUTPUT_FROM_FX_CMD: u8 = 0x03;
const OUTPUT_STEREO_LINK_CMD: u8 = 0x04;
const OUTPUT_INVERT_PHASE_CMD: u8 = 0x07;
const OUTPUT_LINE_LEVEL_CMD: u8 = 0x08;

const HPF_ACTIVATE_CMD: u8 = 0x20;
const HPF_CUT_OFF_CMD: u8 = 0x21;
const HPF_ROLL_OFF_CMD: u8 = 0x22;
const EQ_ACTIVATE_CMD: u8 = 0x40;
const EQ_LOW_TYPE_CMD: u8 = 0x41;
const EQ_LOW_GAIN_CMD: u8 = 0x42;
const EQ_LOW_FREQ_CMD: u8 = 0x43;
const EQ_LOW_QUALITY_CMD: u8 = 0x44;
const EQ_MIDDLE_GAIN_CMD: u8 = 0x45;
const EQ_MIDDLE_FREQ_CMD: u8 = 0x46;
const EQ_MIDDLE_QUALITY_CMD: u8 = 0x47;
const EQ_HIGH_TYPE_CMD: u8 = 0x48;
const EQ_HIGH_GAIN_CMD: u8 = 0x49;
const EQ_HIGH_FREQ_CMD: u8 = 0x4a;
const EQ_HIGH_QUALITY_CMD: u8 = 0x4b;
const DYN_ACTIVATE_CMD: u8 = 0x60;
const DYN_GAIN_CMD: u8 = 0x61;
const DYN_ATTACK_CMD: u8 = 0x62;
const DYN_RELEASE_CMD: u8 = 0x63;
const DYN_COMPR_THRESHOLD_CMD: u8 = 0x64;
const DYN_COMPR_RATIO_CMD: u8 = 0x65;
const DYN_EXPANDER_THRESHOLD_CMD: u8 = 0x66;
const DYN_EXPANDER_RATIO_CMD: u8 = 0x67;
const AUTOLEVEL_ACTIVATE_CMD: u8 = 0x80;
const AUTOLEVEL_MAX_GAIN_CMD: u8 = 0x81;
const AUTOLEVEL_HEADROOM_CMD: u8 = 0x82;
const AUTOLEVEL_RISE_TIME_CMD: u8 = 0x83;

const FX_REVERB_ACTIVATE_CMD: u8 = 0x00;
const FX_REVERB_TYPE_CMD: u8 = 0x01;
const FX_REVERB_PRE_DELAY_CMD: u8 = 0x02;
const FX_REVERB_PRE_HPF_FREQ_CMD: u8 = 0x03;
const FX_REVERB_ROOM_SCALE_CMD: u8 = 0x04;
const FX_REVERB_ATTACK_CMD: u8 = 0x05;
const FX_REVERB_HOLD_CMD: u8 = 0x06;
const FX_REVERB_RELEASE_CMD: u8 = 0x07;
const FX_REVERB_POST_LPF_FREQ_CMD: u8 = 0x08;
const FX_REVERB_TIME_CMD: u8 = 0x09;
const FX_REVERB_DAMPING_FREQ_CMD: u8 = 0x0a;
const FX_REVERB_SMOOTH_CMD: u8 = 0x0b;
const FX_REVERB_VOLUME_CMD: u8 = 0x0c;
const FX_REVERB_STEREO_WIDTH_CMD: u8 = 0x0d;

const FX_ECHO_ACTIVATE_CMD: u8 = 0x20;
const FX_ECHO_TYPE_CMD: u8 = 0x21;
const FX_ECHO_DELAY_CMD: u8 = 0x22;
const FX_ECHO_FEEDBACK_CMD: u8 = 0x23;
const FX_ECHO_LPF_FREQ_CMD: u8 = 0x24;
const FX_ECHO_VOLUME_CMD: u8 = 0x25;
const FX_ECHO_STEREO_WIDTH_CMD: u8 = 0x26;

/// Nominal level of analog input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatterInNominalLevel {
    Low,
    /// +4 dBu.
    Professional,
}

impl Default for LatterInNominalLevel {
    fn default() -> Self {
        Self::Low
    }
}

fn deserialize_input_nominal_level(level: &LatterInNominalLevel) -> i16 {
    match level {
        LatterInNominalLevel::Low => 0x0000,
        LatterInNominalLevel::Professional => 0x0001,
    }
}

/// State of inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterInputState {
    /// Whether to link each pair of left and right ports.
    pub stereo_links: Vec<bool>,
    /// Whether to inverse the phase of analog, spdif, and adat inputs.
    pub invert_phases: Vec<bool>,
    /// The gain of analog line input. The value is between 0 and 120 to represent 0.00 dB and 12.00 dB.
    pub line_gains: Vec<i16>,
    /// The nominal level of analog line input.
    pub line_levels: Vec<LatterInNominalLevel>,
    /// Whether to enable powering for mic input. This setting has no effect when the microphone is
    /// used for instrument.
    pub mic_powers: Vec<bool>,
    /// Whether to use mic input for instrument.
    pub mic_insts: Vec<bool>,
}

/// The specification of inputs.
pub trait RmeFfLatterInputSpecification: RmeFfLatterDspSpecification {
    /// The minimum value of physical inputs.
    const PHYS_INPUT_GAIN_MIN: i32 = 0;
    /// The maximum value of physical inputs.
    const PHYS_INPUT_GAIN_MAX: i32 = 120;
    /// The step value of physical inputs.
    const PHYS_INPUT_GAIN_STEP: i32 = 1;

    /// Instantiate input parameters.
    fn create_input_parameters() -> FfLatterInputState {
        FfLatterInputState {
            stereo_links: vec![Default::default(); Self::PHYS_INPUT_COUNT / 2],
            invert_phases: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            line_gains: vec![Default::default(); Self::LINE_INPUT_COUNT],
            line_levels: vec![Default::default(); Self::LINE_INPUT_COUNT],
            mic_powers: vec![Default::default(); Self::MIC_INPUT_COUNT],
            mic_insts: vec![Default::default(); Self::MIC_INPUT_COUNT],
        }
    }

    /// Instantiate input equalizer parameters.
    fn create_input_hpf_parameters() -> FfLatterInputHpfParameters {
        FfLatterInputHpfParameters(FfLatterHpfState {
            activates: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            cut_offs: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            roll_offs: vec![Default::default(); Self::PHYS_INPUT_COUNT],
        })
    }

    /// Instantiate input equalizer parameters.
    fn create_input_equalizer_parameters() -> FfLatterInputEqualizerParameters {
        FfLatterInputEqualizerParameters(FfLatterEqState {
            activates: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            low_types: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            low_gains: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            low_freqs: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            low_qualities: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            middle_gains: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            middle_freqs: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            middle_qualities: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            high_types: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            high_gains: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            high_freqs: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            high_qualities: vec![Default::default(); Self::PHYS_INPUT_COUNT],
        })
    }

    /// Instantiate input dynamics parameters.
    fn create_input_dynamics_parameters() -> FfLatterInputDynamicsParameters {
        FfLatterInputDynamicsParameters(FfLatterDynState {
            activates: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            gains: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            attacks: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            releases: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            compressor_thresholds: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            compressor_ratios: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            expander_thresholds: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            expander_ratios: vec![Default::default(); Self::PHYS_INPUT_COUNT],
        })
    }

    /// Instantiate input autolevel parameters.
    fn create_input_autolevel_parameters() -> FfLatterInputAutolevelParameters {
        FfLatterInputAutolevelParameters(FfLatterAutolevelState {
            activates: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            max_gains: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            headrooms: vec![Default::default(); Self::PHYS_INPUT_COUNT],
            rise_times: vec![Default::default(); Self::PHYS_INPUT_COUNT],
        })
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterInputSpecification for O {}

impl<O: RmeFfLatterInputSpecification> RmeFfCommandParamsSerialize<FfLatterInputState> for O {
    fn serialize_commands(state: &FfLatterInputState) -> Vec<u32> {
        assert_eq!(state.stereo_links.len(), Self::PHYS_INPUT_COUNT / 2);
        assert_eq!(state.invert_phases.len(), Self::PHYS_INPUT_COUNT);
        assert_eq!(state.line_gains.len(), Self::LINE_INPUT_COUNT);
        assert_eq!(state.line_levels.len(), Self::LINE_INPUT_COUNT);
        assert_eq!(state.mic_powers.len(), Self::MIC_INPUT_COUNT);
        assert_eq!(state.mic_insts.len(), Self::MIC_INPUT_COUNT);

        let mut cmds = Vec::new();

        state
            .stereo_links
            .iter()
            .enumerate()
            .for_each(|(i, &link)| {
                let ch = (i * 2) as u8;
                cmds.push(create_phys_port_cmd(ch, INPUT_STEREO_LINK_CMD, link as i16));
            });

        state
            .invert_phases
            .iter()
            .enumerate()
            .for_each(|(i, &invert_phase)| {
                let ch = i as u8;
                cmds.push(create_phys_port_cmd(
                    ch,
                    INPUT_INVERT_PHASE_CMD,
                    invert_phase as i16,
                ));
            });

        state.line_gains.iter().enumerate().for_each(|(i, &gain)| {
            let ch = (Self::LINE_INPUT_OFFSET + i) as u8;
            cmds.push(create_phys_port_cmd(ch, INPUT_LINE_GAIN_CMD, gain as i16));
        });

        state.line_levels.iter().enumerate().for_each(|(i, level)| {
            let ch = (Self::LINE_INPUT_OFFSET + i) as u8;
            cmds.push(create_phys_port_cmd(
                ch,
                INPUT_LINE_LEVEL_CMD,
                deserialize_input_nominal_level(level),
            ));
        });

        (0..Self::MIC_INPUT_COUNT).for_each(|i| {
            // NOTE: The offset is required for microphone inputs.
            let ch = (Self::MIC_INPUT_OFFSET + i) as u8;

            let cmd = create_phys_port_cmd(ch, INPUT_MIC_INST_CMD, state.mic_insts[i] as i16);
            cmds.push(cmd);

            // NOTE: When enabling the setting for instrument, the setting for phantom powering is
            // disabled automatically.
            let powering = state.mic_powers[i] && !state.mic_insts[i];
            let cmd = create_phys_port_cmd(ch, INPUT_MIC_POWER_CMD, powering as i16);
            cmds.push(cmd);
        });

        cmds
    }
}

/// Gain of microphone preamps (command 0x07 on the mic channels).
///
/// The value is in 0.1 dB steps, e.g. 430 means 43 dB. Confirmed on the Fireface UCX, whose
/// front knob offers 0 dB and 10..60 dB. This state is never written wholly at startup, so the
/// device keeps the gain set with the knob until a new value is commanded partially.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterMicGainState {
    /// The gain of each microphone input, in 0.1 dB.
    pub gains: Vec<i16>,
}

/// The specification of microphone preamp gain.
pub trait RmeFfLatterMicGainSpecification: RmeFfLatterDspSpecification {
    /// The minimum gain, in 0.1 dB.
    const MIC_INPUT_GAIN_MIN: i32 = 0;
    /// The maximum gain, in 0.1 dB.
    const MIC_INPUT_GAIN_MAX: i32 = 600;
    /// The step of gain, in 0.1 dB.
    const MIC_INPUT_GAIN_STEP: i32 = 10;

    /// Instantiate microphone gain parameters. The values are unknown until commanded, so each
    /// is set to -1 so that any first command differs from it.
    fn create_mic_gain_parameters() -> FfLatterMicGainState {
        FfLatterMicGainState {
            gains: vec![-1; Self::MIC_INPUT_COUNT],
        }
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterMicGainSpecification for O {}

impl<O: RmeFfLatterMicGainSpecification> RmeFfCommandParamsSerialize<FfLatterMicGainState> for O {
    fn serialize_commands(state: &FfLatterMicGainState) -> Vec<u32> {
        assert_eq!(state.gains.len(), Self::MIC_INPUT_COUNT);

        state
            .gains
            .iter()
            .enumerate()
            .map(|(i, &gain)| {
                let ch = (Self::MIC_INPUT_OFFSET + i) as u8;
                create_phys_port_cmd(ch, INPUT_LINE_GAIN_CMD, gain)
            })
            .collect()
    }
}

fn deserialize_output_nominal_level(level: &LineOutNominalLevel) -> i16 {
    match level {
        LineOutNominalLevel::Consumer => 0x0000,
        LineOutNominalLevel::Professional => 0x0001,
        LineOutNominalLevel::High => 0x0002,
    }
}

/// The specification of output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterOutputState {
    /// The level of volume. Each value is between -650 (0xfd76) and 60 (0x003c) to represent
    /// -65.00 dB and 6.00 dB.
    pub vols: Vec<i16>,
    /// The balance between left and right. The value is between -100 (0xff9c) and 100 (0x0064).
    pub stereo_balance: Vec<i16>,
    /// Whether to link each pair of left and right ports.
    pub stereo_links: Vec<bool>,
    /// Whether to inverse the phase of analog, spdif, and adat outputs.
    pub invert_phases: Vec<bool>,
    /// The nominal level of analog line output.
    pub line_levels: Vec<LineOutNominalLevel>,
}

/// The specification of output.
pub trait RmeFfLatterOutputSpecification: RmeFfLatterDspSpecification {
    /// The minimum value for volume of physical output.
    const PHYS_OUTPUT_VOL_MIN: i32 = -650;
    /// The default value for volume of physical output.
    const PHYS_OUTPUT_VOL_ZERO: i32 = 0;
    /// The maximum value for volume of physical output.
    const PHYS_OUTPUT_VOL_MAX: i32 = 60;
    /// The step value of for volume physical output.
    const PHYS_OUTPUT_VOL_STEP: i32 = 1;

    /// The minimum value for left and right balance of physical output.
    const PHYS_OUTPUT_BALANCE_MIN: i32 = -100;
    /// The maximum value for left and right balance of physical output.
    const PHYS_OUTPUT_BALANCE_MAX: i32 = 100;
    /// The step value for left and right balance of physical output.
    const PHYS_OUTPUT_BALANCE_STEP: i32 = 1;

    /// The number of physical outputs.
    const CH_OFFSET: u8 = Self::PHYS_INPUT_COUNT as u8;
    /// The number of physical output pairs.
    const OUTPUT_PAIR_COUNT: usize = Self::OUTPUT_COUNT / 2;

    /// Instantiate output parameters.
    fn create_output_parameters() -> FfLatterOutputState {
        FfLatterOutputState {
            vols: vec![Default::default(); Self::OUTPUT_COUNT],
            stereo_balance: vec![Default::default(); Self::OUTPUT_COUNT / 2],
            stereo_links: vec![Default::default(); Self::OUTPUT_COUNT / 2],
            invert_phases: vec![Default::default(); Self::OUTPUT_COUNT],
            line_levels: vec![Default::default(); Self::LINE_OUTPUT_COUNT],
        }
    }

    /// Instantiate output equalizer parameters.
    fn create_output_hpf_parameters() -> FfLatterOutputHpfParameters {
        FfLatterOutputHpfParameters(FfLatterHpfState {
            activates: vec![Default::default(); Self::OUTPUT_COUNT],
            cut_offs: vec![Default::default(); Self::OUTPUT_COUNT],
            roll_offs: vec![Default::default(); Self::OUTPUT_COUNT],
        })
    }

    /// Instantiate output equalizer parameters.
    fn create_output_equalizer_parameters() -> FfLatterOutputEqualizerParameters {
        FfLatterOutputEqualizerParameters(FfLatterEqState {
            activates: vec![Default::default(); Self::OUTPUT_COUNT],
            low_types: vec![Default::default(); Self::OUTPUT_COUNT],
            low_gains: vec![Default::default(); Self::OUTPUT_COUNT],
            low_freqs: vec![Default::default(); Self::OUTPUT_COUNT],
            low_qualities: vec![Default::default(); Self::OUTPUT_COUNT],
            middle_gains: vec![Default::default(); Self::OUTPUT_COUNT],
            middle_freqs: vec![Default::default(); Self::OUTPUT_COUNT],
            middle_qualities: vec![Default::default(); Self::OUTPUT_COUNT],
            high_types: vec![Default::default(); Self::OUTPUT_COUNT],
            high_gains: vec![Default::default(); Self::OUTPUT_COUNT],
            high_freqs: vec![Default::default(); Self::OUTPUT_COUNT],
            high_qualities: vec![Default::default(); Self::OUTPUT_COUNT],
        })
    }

    /// Instantiate output dynamics parameters.
    fn create_output_dynamics_parameters() -> FfLatterOutputDynamicsParameters {
        FfLatterOutputDynamicsParameters(FfLatterDynState {
            activates: vec![Default::default(); Self::OUTPUT_COUNT],
            gains: vec![Default::default(); Self::OUTPUT_COUNT],
            attacks: vec![Default::default(); Self::OUTPUT_COUNT],
            releases: vec![Default::default(); Self::OUTPUT_COUNT],
            compressor_thresholds: vec![Default::default(); Self::OUTPUT_COUNT],
            compressor_ratios: vec![Default::default(); Self::OUTPUT_COUNT],
            expander_thresholds: vec![Default::default(); Self::OUTPUT_COUNT],
            expander_ratios: vec![Default::default(); Self::OUTPUT_COUNT],
        })
    }

    /// Instantiate output autolevel parameters.
    fn create_output_autolevel_parameters() -> FfLatterOutputAutolevelParameters {
        FfLatterOutputAutolevelParameters(FfLatterAutolevelState {
            activates: vec![Default::default(); Self::OUTPUT_COUNT],
            max_gains: vec![Default::default(); Self::OUTPUT_COUNT],
            headrooms: vec![Default::default(); Self::OUTPUT_COUNT],
            rise_times: vec![Default::default(); Self::OUTPUT_COUNT],
        })
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterOutputSpecification for O {}

impl<O: RmeFfLatterOutputSpecification> RmeFfCommandParamsSerialize<FfLatterOutputState> for O {
    fn serialize_commands(state: &FfLatterOutputState) -> Vec<u32> {
        assert_eq!(state.vols.len(), Self::OUTPUT_COUNT);
        assert_eq!(state.stereo_balance.len(), Self::OUTPUT_COUNT / 2);
        assert_eq!(state.stereo_links.len(), Self::OUTPUT_COUNT / 2);
        assert_eq!(state.invert_phases.len(), Self::OUTPUT_COUNT);
        assert_eq!(state.line_levels.len(), Self::LINE_OUTPUT_COUNT);

        let mut cmds = Vec::new();
        let ch_offset = Self::PHYS_INPUT_COUNT as u8;

        state.vols.iter().enumerate().for_each(|(i, &vol)| {
            let ch = ch_offset + i as u8;
            cmds.push(create_phys_port_cmd(ch, OUTPUT_VOL_CMD, vol));
        });

        state
            .stereo_balance
            .iter()
            .enumerate()
            .for_each(|(i, &balance)| {
                let ch = ch_offset + (i * 2) as u8;
                cmds.push(create_phys_port_cmd(ch, OUTPUT_STEREO_BALANCE_CMD, balance));
            });

        state
            .stereo_links
            .iter()
            .enumerate()
            .for_each(|(i, &link)| {
                let ch = ch_offset + (i * 2) as u8;
                cmds.push(create_phys_port_cmd(
                    ch,
                    OUTPUT_STEREO_LINK_CMD,
                    link as i16,
                ));
            });

        state
            .invert_phases
            .iter()
            .enumerate()
            .for_each(|(i, &invert_phase)| {
                let ch = ch_offset + i as u8;
                cmds.push(create_phys_port_cmd(
                    ch,
                    OUTPUT_INVERT_PHASE_CMD,
                    invert_phase as i16,
                ));
            });

        state
            .line_levels
            .iter()
            .enumerate()
            .for_each(|(i, line_level)| {
                let ch = ch_offset + i as u8;
                cmds.push(create_phys_port_cmd(
                    ch,
                    OUTPUT_LINE_LEVEL_CMD,
                    deserialize_output_nominal_level(line_level),
                ));
            });

        cmds
    }
}

/// State of sources for mixer.
///
/// Each value is between 0x8000 and 0xa000 through 0x9000 to represent -65.00 dB and 6.00 dB
/// through 0.00 dB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterMixer {
    /// The gain of sources from line inputs.
    pub line_gains: Vec<u16>,
    /// The gain of sources from microphone inputs.
    pub mic_gains: Vec<u16>,
    /// The gain of sources from S/PDIF inputs.
    pub spdif_gains: Vec<u16>,
    /// The gain of sources from ADAT inputs.
    pub adat_gains: Vec<u16>,
    /// The gain of sources from stream inputs.
    pub stream_gains: Vec<u16>,
    /// The gain of sources from fx return
    pub fx_return_gains: Vec<u16>,
}

/// State of mixers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterMixerState(pub Vec<FfLatterMixer>);

/// The specification of mixer.
pub trait RmeFfLatterMixerSpecification: RmeFfLatterDspSpecification {
    /// The minimum value of gain for source of mixer.
    const MIXER_INPUT_GAIN_MIN: i32 = 0x8000;
    /// The zero value of gain for source of mixer.
    const MIXER_INPUT_GAIN_ZERO: i32 = 0x9000;
    /// The maximum value of gain for source of mixer.
    const MIXER_INPUT_GAIN_MAX: i32 = 0xa000;
    /// The step value of gain for source of mixer.
    const MIXER_INPUT_GAIN_STEP: i32 = 1;

    fn create_mixer_parameters() -> FfLatterMixerState {
        FfLatterMixerState(vec![
            FfLatterMixer {
                line_gains: vec![Default::default(); Self::LINE_INPUT_COUNT],
                mic_gains: vec![Default::default(); Self::MIC_INPUT_COUNT],
                spdif_gains: vec![Default::default(); Self::SPDIF_INPUT_COUNT],
                adat_gains: vec![Default::default(); Self::ADAT_INPUT_COUNT],
                fx_return_gains: vec![Default::default(); Self::FX_RETURN_COUNT],
                stream_gains: vec![Default::default(); Self::STREAM_INPUT_COUNT],
            };
            Self::OUTPUT_COUNT
        ])
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterMixerSpecification for O {}

impl<O: RmeFfLatterMixerSpecification> RmeFfCommandParamsSerialize<FfLatterMixerState> for O {
    fn serialize_commands(state: &FfLatterMixerState) -> Vec<u32> {
        state
            .0
            .iter()
            .enumerate()
            .flat_map(|(i, mixer)| {
                let index = i as u16;
                // Physical inputs are at channel 0 onward, while fx returns and stream inputs
                // are at fixed offsets just below and from STREAM_OFFSET (0x20) regardless of
                // the number of physical inputs. Contiguous numbering only coincides with this
                // layout for models with 30 physical inputs (802); for UCX (18) it misplaced
                // stream sources at 0x14.
                let phys = phys_input_channels::<Self, _>(
                    &mixer.line_gains,
                    &mixer.mic_gains,
                    mixer.spdif_gains.iter().chain(&mixer.adat_gains),
                )
                .map(|(ch, gain)| (ch as u16, gain));
                let fx_return_offset = Self::STREAM_OFFSET - mixer.fx_return_gains.len() as u16;
                let fx_returns = mixer
                    .fx_return_gains
                    .iter()
                    .enumerate()
                    .map(move |(j, &gain)| (fx_return_offset + j as u16, gain));
                let streams = mixer
                    .stream_gains
                    .iter()
                    .enumerate()
                    .map(|(j, &gain)| (Self::STREAM_OFFSET + j as u16, gain));
                phys.chain(fx_returns)
                    .chain(streams)
                    .map(|(ch, gain)| create_virt_port_cmd(Self::MIXER_STEP, index, ch, gain))
                    .collect::<Vec<u32>>()
            })
            .collect()
    }
}

/// Level of roll off in high pass filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfLatterHpfRollOffLevel {
    L6,
    L12,
    L18,
    L24,
}

impl Default for FfLatterHpfRollOffLevel {
    fn default() -> Self {
        Self::L6
    }
}

fn deserialize_hpf_roll_off_level(freq: &FfLatterHpfRollOffLevel) -> i16 {
    match freq {
        FfLatterHpfRollOffLevel::L6 => 0,
        FfLatterHpfRollOffLevel::L12 => 1,
        FfLatterHpfRollOffLevel::L18 => 2,
        FfLatterHpfRollOffLevel::L24 => 3,
    }
}

/// State of high pass filter in channel strip effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterHpfState {
    /// Whether to activate high pass filter.
    pub activates: Vec<bool>,
    /// The frequency to cut between 20 and 500 Hz.
    pub cut_offs: Vec<u16>,
    /// The ratio to decline gain.
    pub roll_offs: Vec<FfLatterHpfRollOffLevel>,
}

fn hpf_state_to_cmds(state: &FfLatterHpfState, ch_offset: u8) -> Vec<u32> {
    assert_eq!(state.cut_offs.len(), state.activates.len());
    assert_eq!(state.roll_offs.len(), state.activates.len());

    let mut cmds = Vec::new();

    (0..state.activates.len()).for_each(|i| {
        let ch = ch_offset + i as u8;
        cmds.push(create_phys_port_cmd(
            ch,
            HPF_ACTIVATE_CMD,
            state.activates[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            HPF_CUT_OFF_CMD,
            state.cut_offs[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            HPF_ROLL_OFF_CMD,
            deserialize_hpf_roll_off_level(&state.roll_offs[i]),
        ));
    });

    cmds
}

/// Parameters of input high pass filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterInputHpfParameters(pub FfLatterHpfState);

impl AsRef<FfLatterHpfState> for FfLatterInputHpfParameters {
    fn as_ref(&self) -> &FfLatterHpfState {
        &self.0
    }
}

impl AsMut<FfLatterHpfState> for FfLatterInputHpfParameters {
    fn as_mut(&mut self) -> &mut FfLatterHpfState {
        &mut self.0
    }
}

/// Parameters of output high pass filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterOutputHpfParameters(pub FfLatterHpfState);

impl AsRef<FfLatterHpfState> for FfLatterOutputHpfParameters {
    fn as_ref(&self) -> &FfLatterHpfState {
        &self.0
    }
}

impl AsMut<FfLatterHpfState> for FfLatterOutputHpfParameters {
    fn as_mut(&mut self) -> &mut FfLatterHpfState {
        &mut self.0
    }
}

/// The trait for specification of high pass filter.
pub trait RmeFfLatterHpfSpecification {
    /// The minimum value of cut off.
    const HPF_CUT_OFF_MIN: i32 = 20;
    /// The maximum value of cut off.
    const HPF_CUT_OFF_MAX: i32 = 500;
    /// The step value of cut off.
    const HPF_CUT_OFF_STEP: i32 = 1;
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterHpfSpecification for O {}

impl<O> RmeFfCommandParamsSerialize<FfLatterInputHpfParameters> for O
where
    O: RmeFfLatterHpfSpecification + RmeFfLatterInputSpecification,
{
    fn serialize_commands(params: &FfLatterInputHpfParameters) -> Vec<u32> {
        hpf_state_to_cmds(&params.0, 0)
    }
}

impl<O> RmeFfCommandParamsSerialize<FfLatterOutputHpfParameters> for O
where
    O: RmeFfLatterHpfSpecification + RmeFfLatterOutputSpecification,
{
    fn serialize_commands(params: &FfLatterOutputHpfParameters) -> Vec<u32> {
        hpf_state_to_cmds(&params.0, Self::PHYS_INPUT_COUNT as u8)
    }
}

/// Type of bandwidth equalizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfLatterChStripEqType {
    Peak,
    Shelf,
    LowPass,
}

impl Default for FfLatterChStripEqType {
    fn default() -> Self {
        Self::Peak
    }
}

fn deserialize_eq_type(eq_type: &FfLatterChStripEqType) -> i16 {
    match eq_type {
        FfLatterChStripEqType::Peak => 0x0000,
        FfLatterChStripEqType::Shelf => 0x0001,
        FfLatterChStripEqType::LowPass => 0x0002,
    }
}

/// State of equalizer in channel strip effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterEqState {
    /// Whether to activate equalizer.
    pub activates: Vec<bool>,
    /// The type of equalizer at low bandwidth.
    pub low_types: Vec<FfLatterChStripEqType>,
    /// The gain of equalizer at low bandwidth between -20 and 20.
    pub low_gains: Vec<i16>,
    /// The frequency of equalizer at low bandwidth between 20 and 20000.
    pub low_freqs: Vec<u16>,
    /// The quality of equalizer at low bandwidth between 7 and 50, displayed by 1/10.
    pub low_qualities: Vec<u16>,
    /// The gain of equalizer at middle bandwidth between -20 and 20.
    pub middle_gains: Vec<i16>,
    /// The frequency of equalizer at middle bandwidth between 20 and 20000.
    pub middle_freqs: Vec<u16>,
    /// The quality of equalizer at middle bandwidth between 7 and 50, displayed by 1/10.
    pub middle_qualities: Vec<u16>,
    /// The type of equalizer at high bandwidth.
    pub high_types: Vec<FfLatterChStripEqType>,
    /// The gain of equalizer at high bandwidth between -20 and 20.
    pub high_gains: Vec<i16>,
    /// The frequency of equalizer at high bandwidth between 20 and 20000.
    pub high_freqs: Vec<u16>,
    /// The quality of equalizer at high bandwidth between 7 and 50, displayed by 1/10.
    pub high_qualities: Vec<u16>,
}

fn eq_state_to_cmds(state: &FfLatterEqState, ch_offset: u8) -> Vec<u32> {
    assert_eq!(state.low_types.len(), state.activates.len());
    assert_eq!(state.low_gains.len(), state.activates.len());
    assert_eq!(state.low_freqs.len(), state.activates.len());
    assert_eq!(state.low_qualities.len(), state.activates.len());
    assert_eq!(state.middle_gains.len(), state.activates.len());
    assert_eq!(state.middle_freqs.len(), state.activates.len());
    assert_eq!(state.middle_qualities.len(), state.activates.len());
    assert_eq!(state.high_types.len(), state.activates.len());
    assert_eq!(state.high_gains.len(), state.activates.len());
    assert_eq!(state.high_freqs.len(), state.activates.len());
    assert_eq!(state.high_qualities.len(), state.activates.len());

    let mut cmds = Vec::new();

    (0..state.activates.len()).for_each(|i| {
        let ch = ch_offset + i as u8;
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_ACTIVATE_CMD,
            state.activates[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_LOW_TYPE_CMD,
            deserialize_eq_type(&state.low_types[i]),
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_LOW_GAIN_CMD,
            state.low_gains[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_LOW_FREQ_CMD,
            state.low_freqs[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_LOW_QUALITY_CMD,
            state.low_qualities[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_MIDDLE_GAIN_CMD,
            state.middle_gains[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_MIDDLE_FREQ_CMD,
            state.middle_freqs[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_MIDDLE_QUALITY_CMD,
            state.middle_qualities[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_HIGH_TYPE_CMD,
            deserialize_eq_type(&state.high_types[i]),
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_HIGH_GAIN_CMD,
            state.high_gains[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_HIGH_FREQ_CMD,
            state.high_freqs[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            EQ_HIGH_QUALITY_CMD,
            state.high_qualities[i] as i16,
        ));
    });

    cmds
}

/// Parameters of input equalizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterInputEqualizerParameters(pub FfLatterEqState);

impl AsRef<FfLatterEqState> for FfLatterInputEqualizerParameters {
    fn as_ref(&self) -> &FfLatterEqState {
        &self.0
    }
}

impl AsMut<FfLatterEqState> for FfLatterInputEqualizerParameters {
    fn as_mut(&mut self) -> &mut FfLatterEqState {
        &mut self.0
    }
}

/// Parameters of output equalizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterOutputEqualizerParameters(pub FfLatterEqState);

impl AsRef<FfLatterEqState> for FfLatterOutputEqualizerParameters {
    fn as_ref(&self) -> &FfLatterEqState {
        &self.0
    }
}

impl AsMut<FfLatterEqState> for FfLatterOutputEqualizerParameters {
    fn as_mut(&mut self) -> &mut FfLatterEqState {
        &mut self.0
    }
}

/// The trait for specification of equalizer.
pub trait RmeFfLatterEqualizerSpecification {
    /// The minimum value of gain.
    const EQ_GAIN_MIN: i32 = -200;
    /// The maximum value of gain.
    const EQ_GAIN_MAX: i32 = 200;
    /// The step value of gain.
    const EQ_GAIN_STEP: i32 = 1;

    /// The minimum value of frequency.
    const EQ_FREQ_MIN: i32 = 20;
    /// The maximum value of frequency.
    const EQ_FREQ_MAX: i32 = 20000;
    /// The step value of frequency.
    const EQ_FREQ_STEP: i32 = 1;

    /// The minimum value of quality.
    const EQ_QUALITY_MIN: i32 = 7;
    /// The maximum value of quality.
    const EQ_QUALITY_MAX: i32 = 50;
    /// The step value of quality.
    const EQ_QUALITY_STEP: i32 = 1;
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterEqualizerSpecification for O {}

impl<O> RmeFfCommandParamsSerialize<FfLatterInputEqualizerParameters> for O
where
    O: RmeFfLatterEqualizerSpecification + RmeFfLatterInputSpecification,
{
    fn serialize_commands(params: &FfLatterInputEqualizerParameters) -> Vec<u32> {
        eq_state_to_cmds(&params.0, 0)
    }
}

impl<O> RmeFfCommandParamsSerialize<FfLatterOutputEqualizerParameters> for O
where
    O: RmeFfLatterEqualizerSpecification + RmeFfLatterOutputSpecification,
{
    fn serialize_commands(params: &FfLatterOutputEqualizerParameters) -> Vec<u32> {
        eq_state_to_cmds(&params.0, Self::PHYS_INPUT_COUNT as u8)
    }
}

/// State of dynamics in channel strip effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterDynState {
    /// Whether to activate dynamics.
    pub activates: Vec<bool>,
    /// The gain of dynamics between -300 and 300, displayed by 1/10.
    pub gains: Vec<i16>,
    /// The rise time of dynamics between 0 and 200 ms.
    pub attacks: Vec<u16>,
    /// The release time of dynamics between 100 and 999 ms.
    pub releases: Vec<u16>,
    /// The threshold of compressor between -600 and 0, displayed by 1/10.
    pub compressor_thresholds: Vec<i16>,
    /// The ratio of compressor between 10 and 100.
    pub compressor_ratios: Vec<u16>,
    /// The threshold of expander between -990 and -200, displayed by 1/10.
    pub expander_thresholds: Vec<i16>,
    /// The ratio of expander between 10 and 100.
    pub expander_ratios: Vec<u16>,
}

fn dyn_state_to_cmds(state: &FfLatterDynState, ch_offset: u8) -> Vec<u32> {
    assert_eq!(state.gains.len(), state.activates.len());
    assert_eq!(state.attacks.len(), state.activates.len());
    assert_eq!(state.releases.len(), state.activates.len());
    assert_eq!(state.compressor_thresholds.len(), state.activates.len());
    assert_eq!(state.compressor_ratios.len(), state.activates.len());
    assert_eq!(state.expander_thresholds.len(), state.activates.len());
    assert_eq!(state.expander_ratios.len(), state.activates.len());

    let mut cmds = Vec::new();

    (0..state.activates.len()).for_each(|i| {
        let ch = ch_offset + i as u8;
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_ACTIVATE_CMD,
            state.activates[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_GAIN_CMD,
            state.gains[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_ATTACK_CMD,
            state.attacks[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_RELEASE_CMD,
            state.releases[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_COMPR_THRESHOLD_CMD,
            state.compressor_thresholds[i],
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_COMPR_RATIO_CMD,
            state.compressor_ratios[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_EXPANDER_THRESHOLD_CMD,
            state.expander_thresholds[i],
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            DYN_EXPANDER_RATIO_CMD,
            state.expander_ratios[i] as i16,
        ));
    });

    cmds
}

/// Parameters of input dynamics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterInputDynamicsParameters(pub FfLatterDynState);

impl AsRef<FfLatterDynState> for FfLatterInputDynamicsParameters {
    fn as_ref(&self) -> &FfLatterDynState {
        &self.0
    }
}

impl AsMut<FfLatterDynState> for FfLatterInputDynamicsParameters {
    fn as_mut(&mut self) -> &mut FfLatterDynState {
        &mut self.0
    }
}

/// Parameters of output dynamics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterOutputDynamicsParameters(pub FfLatterDynState);

impl AsRef<FfLatterDynState> for FfLatterOutputDynamicsParameters {
    fn as_ref(&self) -> &FfLatterDynState {
        &self.0
    }
}

impl AsMut<FfLatterDynState> for FfLatterOutputDynamicsParameters {
    fn as_mut(&mut self) -> &mut FfLatterDynState {
        &mut self.0
    }
}

/// The specification of channel strip.
pub trait RmeFfLatterDynamicsSpecification {
    /// The minimum value of gain.
    const DYN_GAIN_MIN: i32 = -300;
    /// The maximum value of gain.
    const DYN_GAIN_MAX: i32 = 300;
    /// The step value of gain.
    const DYN_GAIN_STEP: i32 = 1;

    /// The minimum value of attack.
    const DYN_ATTACK_MIN: i32 = 0;
    /// The maximum value of attack.
    const DYN_ATTACK_MAX: i32 = 200;
    /// The step value of attack.
    const DYN_ATTACK_STEP: i32 = 1;

    /// The minimum value of release.
    const DYN_RELEASE_MIN: i32 = 100;
    /// The maximum value of release.
    const DYN_RELEASE_MAX: i32 = 999;
    /// The step value of release.
    const DYN_RELEASE_STEP: i32 = 1;

    /// The minimum value of compressor threshold.
    const DYN_COMP_THRESHOLD_MIN: i32 = -600;
    /// The maximum value of compressor threshold.
    const DYN_COMP_THRESHOLD_MAX: i32 = 0;
    /// The step value of compressor threshold.
    const DYN_COMP_THRESHOLD_STEP: i32 = 1;

    /// The minimum value of ratio.
    const DYN_RATIO_MIN: i32 = 10;
    /// The maximum value of ratio.
    const DYN_RATIO_MAX: i32 = 100;
    /// The step value of ratio.
    const DYN_RATIO_STEP: i32 = 1;

    /// The minimum value of extra threshold.
    const DYN_EX_THRESHOLD_MIN: i32 = -999;
    /// The maximum value of extra .
    const DYN_EX_THRESHOLD_MAX: i32 = -200;
    /// The step value of extra .
    const DYN_EX_THRESHOLD_STEP: i32 = 1;
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterDynamicsSpecification for O {}

impl<O> RmeFfCommandParamsSerialize<FfLatterInputDynamicsParameters> for O
where
    O: RmeFfLatterDynamicsSpecification + RmeFfLatterInputSpecification,
{
    fn serialize_commands(params: &FfLatterInputDynamicsParameters) -> Vec<u32> {
        dyn_state_to_cmds(&params.0, 0)
    }
}

impl<O> RmeFfCommandParamsSerialize<FfLatterOutputDynamicsParameters> for O
where
    O: RmeFfLatterDynamicsSpecification + RmeFfLatterOutputSpecification,
{
    fn serialize_commands(params: &FfLatterOutputDynamicsParameters) -> Vec<u32> {
        dyn_state_to_cmds(&params.0, Self::PHYS_INPUT_COUNT as u8)
    }
}

/// State of autolevel in channel strip effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterAutolevelState {
    /// Whether to activate auto level.
    pub activates: Vec<bool>,
    /// The maximum level of amplification between 0 and 180, displayed by 1/10 for dB.
    pub max_gains: Vec<u16>,
    /// The level of head room to decline signal peak between 30 and 120, displayed by 1/10 for dB.
    pub headrooms: Vec<u16>,
    /// The speed of level increase between 1 and 99, displayed by 1/10 for seconds.
    pub rise_times: Vec<u16>,
}

fn autolevel_state_to_cmds(state: &FfLatterAutolevelState, ch_offset: u8) -> Vec<u32> {
    assert_eq!(state.max_gains.len(), state.activates.len());
    assert_eq!(state.headrooms.len(), state.activates.len());
    assert_eq!(state.rise_times.len(), state.activates.len());

    let mut cmds = Vec::new();

    (0..state.activates.len()).for_each(|i| {
        let ch = ch_offset + i as u8;
        cmds.push(create_phys_port_cmd(
            ch,
            AUTOLEVEL_ACTIVATE_CMD,
            state.activates[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            AUTOLEVEL_MAX_GAIN_CMD,
            state.max_gains[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            AUTOLEVEL_HEADROOM_CMD,
            state.headrooms[i] as i16,
        ));
        cmds.push(create_phys_port_cmd(
            ch,
            AUTOLEVEL_RISE_TIME_CMD,
            state.rise_times[i] as i16,
        ));
    });

    cmds
}

/// Parameters of input autolevel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterInputAutolevelParameters(pub FfLatterAutolevelState);

impl AsRef<FfLatterAutolevelState> for FfLatterInputAutolevelParameters {
    fn as_ref(&self) -> &FfLatterAutolevelState {
        &self.0
    }
}

impl AsMut<FfLatterAutolevelState> for FfLatterInputAutolevelParameters {
    fn as_mut(&mut self) -> &mut FfLatterAutolevelState {
        &mut self.0
    }
}

/// Parameters of output autolevel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterOutputAutolevelParameters(pub FfLatterAutolevelState);

impl AsRef<FfLatterAutolevelState> for FfLatterOutputAutolevelParameters {
    fn as_ref(&self) -> &FfLatterAutolevelState {
        &self.0
    }
}

impl AsMut<FfLatterAutolevelState> for FfLatterOutputAutolevelParameters {
    fn as_mut(&mut self) -> &mut FfLatterAutolevelState {
        &mut self.0
    }
}

/// The specification of autolevel in channel strip effects.
pub trait RmeFfLatterAutolevelSpecification {
    /// The minimum value of max gain.
    const AUTOLEVEL_MAX_GAIN_MIN: i32 = 0;
    /// The maximum value of max gain.
    const AUTOLEVEL_MAX_GAIN_MAX: i32 = 180;
    /// The step value of max gain.
    const AUTOLEVEL_MAX_GAIN_STEP: i32 = 1;

    /// The minimum value of head room.
    const AUTOLEVEL_HEAD_ROOM_MIN: i32 = 30;
    /// The maximum value of head room.
    const AUTOLEVEL_HEAD_ROOM_MAX: i32 = 120;
    /// The step value of head room.
    const AUTOLEVEL_HEAD_ROOM_STEP: i32 = 1;

    /// The minimum value of rise time.
    const AUTOLEVEL_RISE_TIME_MIN: i32 = 1;
    /// The maximum value of rise time.
    const AUTOLEVEL_RISE_TIME_MAX: i32 = 99;
    /// The step value of rise time.
    const AUTOLEVEL_RISE_TIME_STEP: i32 = 1;
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterAutolevelSpecification for O {}

impl<O> RmeFfCommandParamsSerialize<FfLatterInputAutolevelParameters> for O
where
    O: RmeFfLatterAutolevelSpecification + RmeFfLatterInputSpecification,
{
    fn serialize_commands(params: &FfLatterInputAutolevelParameters) -> Vec<u32> {
        autolevel_state_to_cmds(&params.0, 0)
    }
}

impl<O> RmeFfCommandParamsSerialize<FfLatterOutputAutolevelParameters> for O
where
    O: RmeFfLatterAutolevelSpecification + RmeFfLatterOutputSpecification,
{
    fn serialize_commands(params: &FfLatterOutputAutolevelParameters) -> Vec<u32> {
        autolevel_state_to_cmds(&params.0, Self::PHYS_INPUT_COUNT as u8)
    }
}

/// Parameters of sources to fx component.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct FfLatterFxSourceParameters {
    /// The gain of line inputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub line_input_gains: Vec<i16>,
    /// The gain of mic inputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub mic_input_gains: Vec<i16>,
    /// The gain of S/PDIF inputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub spdif_input_gains: Vec<i16>,
    /// The gain of ADAT inputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub adat_input_gains: Vec<i16>,
    /// The gain of stream inputs. Each value is between 0x0000 (-65.0 dB) and 0x8b5c (0.0 dB).
    pub stream_input_gains: Vec<u16>,
}

const FX_MIXER_0: u16 = 0x1e;
const FX_MIXER_1: u16 = 0x1f;

impl<O> RmeFfCommandParamsSerialize<FfLatterFxSourceParameters> for O
where
    O: RmeFfLatterFxSpecification,
{
    fn serialize_commands(params: &FfLatterFxSourceParameters) -> Vec<u32> {
        let mut cmds = Vec::new();

        phys_input_channels::<Self, _>(
            &params.line_input_gains,
            &params.mic_input_gains,
            params.spdif_input_gains.iter().chain(&params.adat_input_gains),
        )
        .for_each(|(ch, gain)| {
            cmds.push(create_phys_port_cmd(ch as u8, INPUT_TO_FX_CMD, gain));
        });

        params
            .stream_input_gains
            .iter()
            .enumerate()
            .for_each(|(i, &gain)| {
                cmds.push(create_virt_port_cmd(
                    Self::MIXER_STEP,
                    FX_MIXER_0,
                    Self::STREAM_OFFSET + i as u16,
                    gain,
                ));
                cmds.push(create_virt_port_cmd(
                    Self::MIXER_STEP,
                    FX_MIXER_1,
                    Self::STREAM_OFFSET + i as u16,
                    gain,
                ));
            });

        cmds
    }
}

/// Parameters of outputs from fx component.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct FfLatterFxOutputParameters {
    /// The volume to line outputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub line_output_vols: Vec<i16>,
    /// The volume to hp outputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub hp_output_vols: Vec<i16>,
    /// The volume to S/PDIF outputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub spdif_output_vols: Vec<i16>,
    /// The volume to ADAT outputs. Each value is between 0xfd76 (-65.0 dB) and 0x0000 (0.0 dB).
    pub adat_output_vols: Vec<i16>,
}

impl<O: RmeFfLatterFxSpecification> RmeFfCommandParamsSerialize<FfLatterFxOutputParameters> for O {
    fn serialize_commands(params: &FfLatterFxOutputParameters) -> Vec<u32> {
        params
            .line_output_vols
            .iter()
            .chain(&params.hp_output_vols)
            .chain(&params.spdif_output_vols)
            .chain(&params.adat_output_vols)
            .enumerate()
            .map(|(i, &gain)| {
                let ch = (Self::PHYS_INPUT_COUNT + i) as u8;
                create_phys_port_cmd(ch, OUTPUT_FROM_FX_CMD, gain)
            })
            .collect()
    }
}

/// Type of reverb effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfLatterFxReverbType {
    SmallRoom,
    MediumRoom,
    LargeRoom,
    Walls,
    Shorty,
    Attack,
    Swagger,
    OldSchool,
    Echoistic,
    EightPlusNine,
    GrandWide,
    Thicker,
    Envelope,
    Gated,
    Space,
}

impl Default for FfLatterFxReverbType {
    fn default() -> Self {
        Self::SmallRoom
    }
}

fn deserialize_fx_reverb_type(reverb_type: &FfLatterFxReverbType) -> i16 {
    match reverb_type {
        FfLatterFxReverbType::SmallRoom => 0x0000,
        FfLatterFxReverbType::MediumRoom => 0x0001,
        FfLatterFxReverbType::LargeRoom => 0x0002,
        FfLatterFxReverbType::Walls => 0x0003,
        FfLatterFxReverbType::Shorty => 0x0004,
        FfLatterFxReverbType::Attack => 0x0005,
        FfLatterFxReverbType::Swagger => 0x0006,
        FfLatterFxReverbType::OldSchool => 0x0007,
        FfLatterFxReverbType::Echoistic => 0x0008,
        FfLatterFxReverbType::EightPlusNine => 0x0009,
        FfLatterFxReverbType::GrandWide => 0x000a,
        FfLatterFxReverbType::Thicker => 0x000b,
        FfLatterFxReverbType::Envelope => 0x000c,
        FfLatterFxReverbType::Gated => 0x000d,
        FfLatterFxReverbType::Space => 0x000e,
    }
}

/// Type of echo effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfLatterFxEchoType {
    StereoEcho,
    StereoCross,
    PongEcho,
}

impl Default for FfLatterFxEchoType {
    fn default() -> Self {
        Self::StereoEcho
    }
}

fn deserialize_fx_echo_type(echo_type: &FfLatterFxEchoType) -> i16 {
    match echo_type {
        FfLatterFxEchoType::StereoEcho => 0x0000,
        FfLatterFxEchoType::StereoCross => 0x0001,
        FfLatterFxEchoType::PongEcho => 0x0002,
    }
}

/// Frequency of low pass filter for echo effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfLatterFxEchoLpfFreq {
    Off,
    H2000,
    H4000,
    H8000,
    H12000,
    H16000,
}

impl Default for FfLatterFxEchoLpfFreq {
    fn default() -> Self {
        Self::Off
    }
}

fn deserialize_fx_echo_lpf_freq(freq: &FfLatterFxEchoLpfFreq) -> i16 {
    match freq {
        FfLatterFxEchoLpfFreq::Off => 0x0000,
        FfLatterFxEchoLpfFreq::H2000 => 0x0005,
        FfLatterFxEchoLpfFreq::H4000 => 0x0004,
        FfLatterFxEchoLpfFreq::H8000 => 0x0003,
        FfLatterFxEchoLpfFreq::H12000 => 0x0002,
        FfLatterFxEchoLpfFreq::H16000 => 0x0001,
    }
}

/// State of reverb in send effect.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FfLatterFxReverbState {
    /// Whether to activate reverb effect.
    pub activate: bool,
    /// The type of reverb effect.
    pub reverb_type: FfLatterFxReverbType,
    /// The pre-delay of reverb effect between 0 and 999.
    pub pre_delay: u16,
    /// The frequency of high pass filter before reverb generation between 20 and 500 Hz.
    pub pre_hpf: u16,
    /// The scale of room between 50 and 300, displayed by 1/10.
    pub room_scale: u16,
    /// The time for increase volume between 5 and 400 ms.
    pub attack: u16,
    /// The time for fixed volume between 5 and 400 ms.
    pub hold: u16,
    /// The time for volume decrease between 5 and 500 ms.
    pub release: u16,
    /// The frequency of low pass filter after reverb generation between 200 and 20000 Hz.
    pub post_lpf: u16,
    /// The time for volume drop between 1 and 49, displayed by 1/10 sec.
    pub time: u16,
    /// The frequency of treble dampling for reverb generation between 2000 and 20000 Hz.
    pub damping: u16,
    /// The level of softener between 0 and 100.
    pub smooth: u16,
    /// The level of output between -650 and 60, displayed by 1/10.
    pub volume: i16,
    /// The stereo width between 0(monaural) and 100(stereo).
    pub stereo_width: u16,
}

const FX_CH: u8 = 0x3c;

fn reverb_state_to_cmds(state: &FfLatterFxReverbState) -> Vec<u32> {
    let mut cmds = Vec::new();

    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_ACTIVATE_CMD,
        state.activate as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_TYPE_CMD,
        deserialize_fx_reverb_type(&state.reverb_type),
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_PRE_DELAY_CMD,
        state.pre_delay as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_PRE_HPF_FREQ_CMD,
        state.pre_hpf as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_ROOM_SCALE_CMD,
        state.room_scale as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_ATTACK_CMD,
        state.attack as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_HOLD_CMD,
        state.hold as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_RELEASE_CMD,
        state.release as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_POST_LPF_FREQ_CMD,
        state.post_lpf as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_TIME_CMD,
        state.time as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_DAMPING_FREQ_CMD,
        state.damping as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_SMOOTH_CMD,
        state.smooth as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_VOLUME_CMD,
        state.volume,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_REVERB_STEREO_WIDTH_CMD,
        state.stereo_width as i16,
    ));

    cmds
}

impl<O: RmeFfLatterFxSpecification> RmeFfCommandParamsSerialize<FfLatterFxReverbState> for O {
    fn serialize_commands(params: &FfLatterFxReverbState) -> Vec<u32> {
        reverb_state_to_cmds(params)
    }
}

/// State of echo in send effect.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct FfLatterFxEchoState {
    /// Whether to activate echo effect.
    pub activate: bool,
    /// The type of echo effect.
    pub echo_type: FfLatterFxEchoType,
    /// The time to delay for echo between 0 and 100.
    pub delay: u16,
    /// The level of feedback for further echo between 0 and 100.
    pub feedback: u16,
    /// The frequency of low pass filter.
    pub lpf: FfLatterFxEchoLpfFreq,
    /// The level of output between -650 and 0, displayed by 1/10.
    pub volume: i16,
    /// The stereo width between 0(monaural) and 100(stereo).
    pub stereo_width: u16,
}

fn echo_state_to_cmds(state: &FfLatterFxEchoState) -> Vec<u32> {
    let mut cmds = Vec::new();

    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_ECHO_ACTIVATE_CMD,
        state.activate as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_ECHO_TYPE_CMD,
        deserialize_fx_echo_type(&state.echo_type),
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_ECHO_DELAY_CMD,
        state.delay as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_ECHO_FEEDBACK_CMD,
        state.feedback as i16,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_ECHO_LPF_FREQ_CMD,
        deserialize_fx_echo_lpf_freq(&state.lpf),
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_ECHO_VOLUME_CMD,
        state.volume,
    ));
    cmds.push(create_phys_port_cmd(
        FX_CH,
        FX_ECHO_STEREO_WIDTH_CMD,
        state.stereo_width as i16,
    ));

    cmds
}

impl<O: RmeFfLatterFxSpecification> RmeFfCommandParamsSerialize<FfLatterFxEchoState> for O {
    fn serialize_commands(params: &FfLatterFxEchoState) -> Vec<u32> {
        echo_state_to_cmds(params)
    }
}

/// The specification of FX.
pub trait RmeFfLatterFxSpecification: RmeFfLatterDspSpecification {
    /// The minimum value for .
    const FX_PHYS_LEVEL_MIN: i32 = -650;
    /// The maximum value for .
    const FX_PHYS_LEVEL_MAX: i32 = 0;
    /// The step value for .
    const FX_PHYS_LEVEL_STEP: i32 = 1;

    /// The minimum value for .
    const FX_VIRT_LEVEL_MIN: i32 = 0;
    /// The maximum value for .
    const FX_VIRT_LEVEL_MAX: i32 = 35676;
    /// The step value for .
    const FX_VIRT_LEVEL_STEP: i32 = 1;

    /// The minimum value for pre delay.
    const REVERB_PRE_DELAY_MIN: i32 = 0;
    /// The maximum value for pre delay.
    const REVERB_PRE_DELAY_MAX: i32 = 999;
    /// The step value for pre delay.
    const REVERB_PRE_DELAY_STEP: i32 = 1;

    /// The minimum value for attack.
    const REVERB_ATTACK_MIN: i32 = 5;
    /// The maximum value for attack.
    const REVERB_ATTACK_MAX: i32 = 400;
    /// The step value for attack.
    const REVERB_ATTACK_STEP: i32 = 1;

    /// The minimum value for hold.
    const REVERB_HOLD_MIN: i32 = 5;
    /// The maximum value for hold.
    const REVERB_HOLD_MAX: i32 = 400;
    /// The step value for hold.
    const REVERB_HOLD_STEP: i32 = 1;

    /// The minimum value for release.
    const REVERB_RELEASE_MIN: i32 = 5;
    /// The maximum value for release.
    const REVERB_RELEASE_MAX: i32 = 500;
    /// The step value for release.
    const REVERB_RELEASE_STEP: i32 = 1;

    /// The minimum value for post low pass filter.
    const REVERB_POST_LPF_FREQ_MIN: i32 = 200;
    /// The maximum value for post low pass filter.
    const REVERB_POST_LPF_FREQ_MAX: i32 = 20000;
    /// The step value for post low pass filter.
    const REVERB_POST_LPF_FREQ_STEP: i32 = 1;

    /// The minimum value for decay time.
    const REVERB_TIME_MIN: i32 = 1;
    /// The maximum value for decay time.
    const REVERB_TIME_MAX: i32 = 49;
    /// The step value for decay time.
    const REVERB_TIME_STEP: i32 = 1;

    /// The minimum value for damping.
    const REVERB_DAMPING_MIN: i32 = 2000;
    /// The maximum value for damping.
    const REVERB_DAMPING_MAX: i32 = 20000;
    /// The step value for damping.
    const REVERB_DAMPING_STEP: i32 = 1;

    /// The minimum value for smooth.
    const REVERB_SMOOTH_MIN: i32 = 0;
    /// The maximum value for smooth.
    const REVERB_SMOOTH_MAX: i32 = 100;
    /// The step value for smooth.
    const REVERB_SMOOTH_STEP: i32 = 1;

    /// The minimum value for volume.
    const REVERB_VOL_MIN: i32 = -650;
    /// The maximum value for volume.
    const REVERB_VOL_MAX: i32 = 60;
    /// The step value for volume.
    const REVERB_VOL_STEP: i32 = 1;

    /// The minimum value for stereo width.
    const REVERB_STEREO_WIDTH_MIN: i32 = 0;
    /// The maximum value for stereo width.
    const REVERB_STEREO_WIDTH_MAX: i32 = 100;
    /// The step value for stereo width.
    const REVERB_STEREO_WIDTH_STEP: i32 = 1;

    /// The minimum value for echo delay.
    const ECHO_DELAY_MIN: i32 = 0;
    /// The maximum value for echo delay.
    const ECHO_DELAY_MAX: i32 = 100;
    /// The step value for echo delay.
    const ECHO_DELAY_STEP: i32 = 1;

    /// The minimum value for echo feedback.
    const ECHO_FEEDBACK_MIN: i32 = 0;
    /// The maximum value for echo feedback.
    const ECHO_FEEDBACK_MAX: i32 = 100;
    /// The step value for echo feedback.
    const ECHO_FEEDBACK_STEP: i32 = 1;

    /// The minimum value for echo volume.
    const ECHO_VOL_MIN: i32 = -650;
    /// The maximum value for echo volume.
    const ECHO_VOL_MAX: i32 = 0;
    /// The step value for echo volume.
    const ECHO_VOL_STEP: i32 = 1;

    /// The minimum value for echo stereo width.
    const ECHO_STEREO_WIDTH_MIN: i32 = 0;
    /// The maximum value for echo stereo width.
    const ECHO_STEREO_WIDTH_MAX: i32 = 100;
    /// The step value for echo stereo width.
    const ECHO_STEREO_WIDTH_STEP: i32 = 1;

    /// Instantiate fx source parameters.
    fn create_fx_sources_parameters() -> FfLatterFxSourceParameters {
        FfLatterFxSourceParameters {
            line_input_gains: vec![0; Self::LINE_INPUT_COUNT],
            mic_input_gains: vec![0; Self::MIC_INPUT_COUNT],
            spdif_input_gains: vec![0; Self::SPDIF_INPUT_COUNT],
            adat_input_gains: vec![0; Self::ADAT_INPUT_COUNT],
            stream_input_gains: vec![0; Self::STREAM_INPUT_COUNT],
        }
    }

    /// Instantiate fx output parameters.
    fn create_fx_output_parameters() -> FfLatterFxOutputParameters {
        FfLatterFxOutputParameters {
            line_output_vols: vec![0; Self::LINE_OUTPUT_COUNT],
            hp_output_vols: vec![0; Self::HP_OUTPUT_COUNT],
            spdif_output_vols: vec![0; Self::SPDIF_OUTPUT_COUNT],
            adat_output_vols: vec![0; Self::ADAT_OUTPUT_COUNT],
        }
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterFxSpecification for O {}

const INPUT_MUTE_CMD: u8 = 0x00;

/// Mute of physical inputs (command 0x00 on the input channels).
///
/// On the Fireface UCX this switch blocks at least the send to the FX bus: with it set, echo and
/// reverb stay silent even with the send and return opened, while the dry signal still reaches
/// the mixer. TotalMix FX calls the parameter "mute". The device keeps the last value, so it is
/// written wholly at startup like the other input parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfLatterInputMuteState {
    /// Whether to mute each physical input.
    pub mutes: Vec<bool>,
}

/// The specification of input mute.
pub trait RmeFfLatterInputMuteSpecification: RmeFfLatterDspSpecification {
    /// Instantiate input mute parameters, all unmuted.
    fn create_input_mute_parameters() -> FfLatterInputMuteState {
        FfLatterInputMuteState {
            mutes: vec![false; Self::PHYS_INPUT_COUNT],
        }
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterInputMuteSpecification for O {}

impl<O: RmeFfLatterInputMuteSpecification> RmeFfCommandParamsSerialize<FfLatterInputMuteState>
    for O
{
    fn serialize_commands(state: &FfLatterInputMuteState) -> Vec<u32> {
        assert_eq!(state.mutes.len(), Self::PHYS_INPUT_COUNT);

        state
            .mutes
            .iter()
            .enumerate()
            .map(|(i, &mute)| create_phys_port_cmd(i as u8, INPUT_MUTE_CMD, mute as i16))
            .collect()
    }
}

// The DSP reports parameter changes, e.g. by the gain knob, through a FIFO. The host has to write
// a heartbeat command periodically, otherwise nothing is reported. TotalMix FX writes it every
// about 20 ms when it has no other command to send.
const DSP_FIFO_COUNT_OFFSET: u64 = 0xffffff200000;
const DSP_FIFO_DATA_OFFSET: u64 = 0xffffff100000;
const DSP_FIFO_MAX_SIZE: usize = 0x400;
const DSP_HEARTBEAT_CMD: u32 = 0x3f000000;
// Request the DSP to report the current value of every parameter.
const DSP_STATE_DUMP_CMD: u32 = 0x3f990812;
const DSP_STATUS_CH: u8 = 0x3f;

/// A parameter value reported by the DSP. It has the same layout as a command for physical port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FfLatterDspReport {
    /// The channel.
    pub ch: u8,
    /// The command code.
    pub cmd: u8,
    /// The value.
    pub value: i16,
}

impl FfLatterDspReport {
    /// Parse a word read from the FIFO. Words with wrong parity, words for virtual ports, and
    /// status words (channel 0x3f) are dropped.
    pub fn parse(word: u32) -> Option<Self> {
        if word.count_ones() % 2 == 0 || word & VIRT_PORT_CMD_FLAG > 0 {
            return None;
        }
        let ch = ((word >> 24) & 0x3f) as u8;
        if ch == DSP_STATUS_CH {
            return None;
        }
        Some(Self {
            ch,
            cmd: ((word >> 16) & 0xff) as u8,
            value: (word & 0xffff) as u16 as i16,
        })
    }
}

/// Operation for parameter reports by the DSP.
pub trait RmeFfLatterDspReportOperation: RmeFfLatterDspSpecification {
    /// Write the heartbeat. The counter is masked to 4 bits.
    fn send_dsp_heartbeat(
        req: &mut FwReq,
        node: &mut FwNode,
        counter: u8,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        write_dsp_cmd(
            req,
            node,
            DSP_HEARTBEAT_CMD | (counter & 0x0f) as u32,
            timeout_ms,
        )
    }

    /// Request the DSP to report every parameter.
    fn request_dsp_state_dump(
        req: &mut FwReq,
        node: &mut FwNode,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        write_dsp_cmd(req, node, DSP_STATE_DUMP_CMD, timeout_ms)
    }

    /// Read the pending reports from the FIFO.
    fn read_dsp_reports(
        req: &mut FwReq,
        node: &mut FwNode,
        timeout_ms: u32,
    ) -> Result<Vec<FfLatterDspReport>, Error> {
        Self::read_dsp_words(req, node, timeout_ms).map(|words| {
            words
                .into_iter()
                .filter_map(FfLatterDspReport::parse)
                .collect()
        })
    }

    /// Read the pending words from the FIFO, including status words.
    fn read_dsp_words(
        req: &mut FwReq,
        node: &mut FwNode,
        timeout_ms: u32,
    ) -> Result<Vec<u32>, Error> {
        let mut raw = [0; 4];
        req.transaction(
            node,
            FwTcode::ReadQuadletRequest,
            DSP_FIFO_COUNT_OFFSET,
            raw.len(),
            &mut raw,
            timeout_ms,
        )?;
        let size = u32::from_le_bytes(raw) as usize;
        if size == 0 || size > DSP_FIFO_MAX_SIZE {
            return Ok(Vec::new());
        }
        let size = size / 4 * 4;
        if size == 0 {
            return Ok(Vec::new());
        }

        let mut raw = vec![0; size];
        req.transaction(
            node,
            FwTcode::ReadBlockRequest,
            DSP_FIFO_DATA_OFFSET,
            raw.len(),
            &mut raw,
            timeout_ms,
        )?;

        Ok(raw
            .chunks_exact(4)
            .map(|q| u32::from_le_bytes([q[0], q[1], q[2], q[3]]))
            .collect())
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterDspReportOperation for O {}

// While the dynamics processor of a channel is active, the DSP reports the level at its detector
// in status words. One word holds the levels of two channels: the higher byte is for the even
// channel, the lower byte for the odd channel. The channels of physical inputs come first, then
// the channels of outputs. When several pairs are active, the DSP reports one pair per FIFO read
// in turn.
const DSP_DYN_METER_CMD_BASE: u8 = 0x10;
const DSP_DYN_METER_CMD_COUNT: u8 = 0x1e;
const DSP_DYN_METER_LEVEL_MAX: i8 = 30;

/// The levels at the detector of dynamics processors, reported by the DSP.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct FfLatterDynamicsMeterState {
    /// For physical inputs. The unit is about 0.832 dB (as TotalMix FX displays it).
    pub inputs: Vec<i8>,
    /// For outputs. The unit is the same.
    pub outputs: Vec<i8>,
}

/// Specification of the levels at the detector of dynamics processors.
pub trait RmeFfLatterDynamicsMeterSpecification: RmeFfLatterDspSpecification {
    /// The level at no signal, or when the DSP reports no level.
    const DYN_LEVEL_MIN: i8 = i8::MIN;
    /// The maximum level.
    const DYN_LEVEL_MAX: i8 = DSP_DYN_METER_LEVEL_MAX;

    fn create_dynamics_meter_state() -> FfLatterDynamicsMeterState {
        FfLatterDynamicsMeterState {
            inputs: vec![Self::DYN_LEVEL_MIN; Self::PHYS_INPUT_COUNT],
            outputs: vec![Self::DYN_LEVEL_MIN; Self::OUTPUT_COUNT],
        }
    }

    /// Update the levels by a word read from the FIFO. Return true when the word has levels.
    fn parse_dynamics_meter_word(state: &mut FfLatterDynamicsMeterState, word: u32) -> bool {
        if word.count_ones() % 2 == 0 || word & VIRT_PORT_CMD_FLAG > 0 {
            return false;
        }
        if ((word >> 24) & 0x3f) as u8 != DSP_STATUS_CH {
            return false;
        }
        let cmd = ((word >> 16) & 0xff) as u8;
        if cmd < DSP_DYN_METER_CMD_BASE || cmd >= DSP_DYN_METER_CMD_BASE + DSP_DYN_METER_CMD_COUNT
        {
            return false;
        }
        let pos = 2 * (cmd - DSP_DYN_METER_CMD_BASE) as usize;
        [(word >> 8) as u8 as i8, word as u8 as i8]
            .iter()
            .enumerate()
            .for_each(|(i, &raw)| {
                // TotalMix FX handles any level above the maximum as no signal.
                let level = if raw > Self::DYN_LEVEL_MAX {
                    Self::DYN_LEVEL_MIN
                } else {
                    raw
                };
                let ch = pos + i;
                let slot = if ch < Self::PHYS_INPUT_COUNT {
                    state.inputs.get_mut(ch)
                } else {
                    state.outputs.get_mut(ch - Self::PHYS_INPUT_COUNT)
                };
                if let Some(slot) = slot {
                    *slot = level;
                }
            });
        true
    }
}

impl<O: RmeFfLatterDspSpecification> RmeFfLatterDynamicsMeterSpecification for O {}

/// Apply a report by the DSP to parameters.
pub trait RmeFfCommandParamsParseReport<T> {
    /// Update the parameters by the report. Return true when any value changed.
    fn parse_report(params: &mut T, report: &FfLatterDspReport) -> bool;
}

fn update_value<V: PartialEq>(slot: Option<&mut V>, value: V) -> bool {
    match slot {
        Some(s) if *s != value => {
            *s = value;
            true
        }
        _ => false,
    }
}

// Index of a value for line/mic inputs, or None.
fn line_input_index<T: RmeFfLatterSpecification>(ch: usize) -> Option<usize> {
    (ch >= T::LINE_INPUT_OFFSET && ch < T::LINE_INPUT_OFFSET + T::LINE_INPUT_COUNT)
        .then(|| ch - T::LINE_INPUT_OFFSET)
}

fn mic_input_index<T: RmeFfLatterSpecification>(ch: usize) -> Option<usize> {
    (ch >= T::MIC_INPUT_OFFSET && ch < T::MIC_INPUT_OFFSET + T::MIC_INPUT_COUNT)
        .then(|| ch - T::MIC_INPUT_OFFSET)
}

// Index of a physical output, or None.
fn output_index<T: RmeFfLatterDspSpecification>(ch: usize) -> Option<usize> {
    (ch >= T::PHYS_INPUT_COUNT && ch < T::PHYS_INPUT_COUNT + T::OUTPUT_COUNT)
        .then(|| ch - T::PHYS_INPUT_COUNT)
}

impl<O: RmeFfLatterInputSpecification> RmeFfCommandParamsParseReport<FfLatterInputState> for O {
    fn parse_report(state: &mut FfLatterInputState, report: &FfLatterDspReport) -> bool {
        let ch = report.ch as usize;
        if ch >= Self::PHYS_INPUT_COUNT {
            return false;
        }
        let on = report.value != 0;
        match report.cmd {
            INPUT_STEREO_LINK_CMD => update_value(state.stereo_links.get_mut(ch / 2), on),
            INPUT_INVERT_PHASE_CMD => update_value(state.invert_phases.get_mut(ch), on),
            INPUT_LINE_GAIN_CMD => line_input_index::<Self>(ch)
                .map(|i| update_value(state.line_gains.get_mut(i), report.value))
                .unwrap_or(false),
            // The same command code is used for nominal level of line inputs and phantom powering
            // of mic inputs.
            INPUT_LINE_LEVEL_CMD => {
                if let Some(i) = line_input_index::<Self>(ch) {
                    let level = match report.value {
                        0 => LatterInNominalLevel::Low,
                        1 => LatterInNominalLevel::Professional,
                        _ => return false,
                    };
                    update_value(state.line_levels.get_mut(i), level)
                } else if let Some(i) = mic_input_index::<Self>(ch) {
                    update_value(state.mic_powers.get_mut(i), on)
                } else {
                    false
                }
            }
            INPUT_MIC_INST_CMD => mic_input_index::<Self>(ch)
                .map(|i| update_value(state.mic_insts.get_mut(i), on))
                .unwrap_or(false),
            _ => false,
        }
    }
}

impl<O: RmeFfLatterMicGainSpecification> RmeFfCommandParamsParseReport<FfLatterMicGainState>
    for O
{
    fn parse_report(state: &mut FfLatterMicGainState, report: &FfLatterDspReport) -> bool {
        if report.cmd != INPUT_LINE_GAIN_CMD {
            return false;
        }
        mic_input_index::<Self>(report.ch as usize)
            .map(|i| update_value(state.gains.get_mut(i), report.value))
            .unwrap_or(false)
    }
}

impl<O: RmeFfLatterInputMuteSpecification> RmeFfCommandParamsParseReport<FfLatterInputMuteState>
    for O
{
    fn parse_report(state: &mut FfLatterInputMuteState, report: &FfLatterDspReport) -> bool {
        if report.cmd != INPUT_MUTE_CMD {
            return false;
        }
        update_value(state.mutes.get_mut(report.ch as usize), report.value != 0)
    }
}

impl<O: RmeFfLatterOutputSpecification> RmeFfCommandParamsParseReport<FfLatterOutputState> for O {
    fn parse_report(state: &mut FfLatterOutputState, report: &FfLatterDspReport) -> bool {
        let i = match output_index::<Self>(report.ch as usize) {
            Some(i) => i,
            None => return false,
        };
        let on = report.value != 0;
        match report.cmd {
            OUTPUT_VOL_CMD => update_value(state.vols.get_mut(i), report.value),
            OUTPUT_STEREO_BALANCE_CMD if i % 2 == 0 => {
                update_value(state.stereo_balance.get_mut(i / 2), report.value)
            }
            OUTPUT_STEREO_LINK_CMD => update_value(state.stereo_links.get_mut(i / 2), on),
            OUTPUT_INVERT_PHASE_CMD => update_value(state.invert_phases.get_mut(i), on),
            OUTPUT_LINE_LEVEL_CMD => {
                let level = match report.value {
                    0 => LineOutNominalLevel::Consumer,
                    1 => LineOutNominalLevel::Professional,
                    2 => LineOutNominalLevel::High,
                    _ => return false,
                };
                update_value(state.line_levels.get_mut(i), level)
            }
            _ => false,
        }
    }
}

impl<O: RmeFfLatterFxSpecification> RmeFfCommandParamsParseReport<FfLatterFxSourceParameters>
    for O
{
    fn parse_report(params: &mut FfLatterFxSourceParameters, report: &FfLatterDspReport) -> bool {
        if report.cmd != INPUT_TO_FX_CMD {
            return false;
        }
        let ch = report.ch as usize;
        let digital_offset = Self::LINE_INPUT_COUNT + Self::MIC_INPUT_COUNT;
        let slot = if let Some(i) = line_input_index::<Self>(ch) {
            params.line_input_gains.get_mut(i)
        } else if let Some(i) = mic_input_index::<Self>(ch) {
            params.mic_input_gains.get_mut(i)
        } else if ch >= digital_offset && ch < Self::PHYS_INPUT_COUNT {
            let i = ch - digital_offset;
            if i < Self::SPDIF_INPUT_COUNT {
                params.spdif_input_gains.get_mut(i)
            } else {
                params.adat_input_gains.get_mut(i - Self::SPDIF_INPUT_COUNT)
            }
        } else {
            None
        };
        update_value(slot, report.value)
    }
}

impl<O: RmeFfLatterFxSpecification> RmeFfCommandParamsParseReport<FfLatterFxOutputParameters>
    for O
{
    fn parse_report(params: &mut FfLatterFxOutputParameters, report: &FfLatterDspReport) -> bool {
        if report.cmd != OUTPUT_FROM_FX_CMD {
            return false;
        }
        let mut i = match output_index::<Self>(report.ch as usize) {
            Some(i) => i,
            None => return false,
        };
        for vols in [
            &mut params.line_output_vols,
            &mut params.hp_output_vols,
            &mut params.spdif_output_vols,
            &mut params.adat_output_vols,
        ] {
            if i < vols.len() {
                return update_value(vols.get_mut(i), report.value);
            }
            i -= vols.len();
        }
        false
    }
}

fn serialize_fx_reverb_type(value: i16) -> Option<FfLatterFxReverbType> {
    [
        FfLatterFxReverbType::SmallRoom,
        FfLatterFxReverbType::MediumRoom,
        FfLatterFxReverbType::LargeRoom,
        FfLatterFxReverbType::Walls,
        FfLatterFxReverbType::Shorty,
        FfLatterFxReverbType::Attack,
        FfLatterFxReverbType::Swagger,
        FfLatterFxReverbType::OldSchool,
        FfLatterFxReverbType::Echoistic,
        FfLatterFxReverbType::EightPlusNine,
        FfLatterFxReverbType::GrandWide,
        FfLatterFxReverbType::Thicker,
        FfLatterFxReverbType::Envelope,
        FfLatterFxReverbType::Gated,
        FfLatterFxReverbType::Space,
    ]
    .iter()
    .copied()
    .find(|t| deserialize_fx_reverb_type(t) == value)
}

impl<O: RmeFfLatterFxSpecification> RmeFfCommandParamsParseReport<FfLatterFxReverbState> for O {
    fn parse_report(state: &mut FfLatterFxReverbState, report: &FfLatterDspReport) -> bool {
        if report.ch != FX_CH {
            return false;
        }
        let v = report.value;
        let u = v as u16;
        match report.cmd {
            FX_REVERB_ACTIVATE_CMD => update_value(Some(&mut state.activate), v != 0),
            FX_REVERB_TYPE_CMD => match serialize_fx_reverb_type(v) {
                Some(t) => update_value(Some(&mut state.reverb_type), t),
                None => false,
            },
            FX_REVERB_PRE_DELAY_CMD => update_value(Some(&mut state.pre_delay), u),
            FX_REVERB_PRE_HPF_FREQ_CMD => update_value(Some(&mut state.pre_hpf), u),
            FX_REVERB_ROOM_SCALE_CMD => update_value(Some(&mut state.room_scale), u),
            FX_REVERB_ATTACK_CMD => update_value(Some(&mut state.attack), u),
            FX_REVERB_HOLD_CMD => update_value(Some(&mut state.hold), u),
            FX_REVERB_RELEASE_CMD => update_value(Some(&mut state.release), u),
            FX_REVERB_POST_LPF_FREQ_CMD => update_value(Some(&mut state.post_lpf), u),
            FX_REVERB_TIME_CMD => update_value(Some(&mut state.time), u),
            FX_REVERB_DAMPING_FREQ_CMD => update_value(Some(&mut state.damping), u),
            FX_REVERB_SMOOTH_CMD => update_value(Some(&mut state.smooth), u),
            FX_REVERB_VOLUME_CMD => update_value(Some(&mut state.volume), v),
            FX_REVERB_STEREO_WIDTH_CMD => update_value(Some(&mut state.stereo_width), u),
            _ => false,
        }
    }
}

impl<O: RmeFfLatterFxSpecification> RmeFfCommandParamsParseReport<FfLatterFxEchoState> for O {
    fn parse_report(state: &mut FfLatterFxEchoState, report: &FfLatterDspReport) -> bool {
        if report.ch != FX_CH {
            return false;
        }
        let v = report.value;
        let u = v as u16;
        match report.cmd {
            FX_ECHO_ACTIVATE_CMD => update_value(Some(&mut state.activate), v != 0),
            FX_ECHO_TYPE_CMD => {
                let t = match v {
                    0 => FfLatterFxEchoType::StereoEcho,
                    1 => FfLatterFxEchoType::StereoCross,
                    2 => FfLatterFxEchoType::PongEcho,
                    _ => return false,
                };
                update_value(Some(&mut state.echo_type), t)
            }
            FX_ECHO_DELAY_CMD => update_value(Some(&mut state.delay), u),
            FX_ECHO_FEEDBACK_CMD => update_value(Some(&mut state.feedback), u),
            FX_ECHO_LPF_FREQ_CMD => {
                let f = match v {
                    0 => FfLatterFxEchoLpfFreq::Off,
                    5 => FfLatterFxEchoLpfFreq::H2000,
                    4 => FfLatterFxEchoLpfFreq::H4000,
                    3 => FfLatterFxEchoLpfFreq::H8000,
                    2 => FfLatterFxEchoLpfFreq::H12000,
                    1 => FfLatterFxEchoLpfFreq::H16000,
                    _ => return false,
                };
                update_value(Some(&mut state.lpf), f)
            }
            FX_ECHO_VOLUME_CMD => update_value(Some(&mut state.volume), v),
            FX_ECHO_STEREO_WIDTH_CMD => update_value(Some(&mut state.stereo_width), u),
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn midi_tx_low_offset_serdes() {
        [
            FfLatterMidiTxLowOffset::A0000,
            FfLatterMidiTxLowOffset::A0080,
            FfLatterMidiTxLowOffset::A0100,
            FfLatterMidiTxLowOffset::A0180,
        ]
        .iter()
        .for_each(|orig| {
            let mut quad = 0;
            serialize_midi_tx_low_offset(orig, &mut quad);
            let mut target = FfLatterMidiTxLowOffset::default();
            deserialize_midi_tx_low_offset(&mut target, &quad);

            assert_eq!(&target, orig);
        });
    }

    #[test]
    fn clock_rate_serdes() {
        [
            ClkNominalRate::R32000,
            ClkNominalRate::R44100,
            ClkNominalRate::R48000,
            ClkNominalRate::R64000,
            ClkNominalRate::R88200,
            ClkNominalRate::R96000,
            ClkNominalRate::R128000,
            ClkNominalRate::R176400,
            ClkNominalRate::R192000,
        ]
        .iter()
        .for_each(|orig| {
            let mut quad = 0;
            serialize_clock_rate(orig, &mut quad, 0);
            let mut target = ClkNominalRate::default();
            deserialize_clock_rate(&mut target, &quad, 0);

            assert_eq!(&target, orig);
        });
    }
}
