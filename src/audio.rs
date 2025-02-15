use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use clap::ValueEnum;

use crate::AudioConfig;

// TODO: add anyhow
pub trait Encoder: Send {
    fn encode(&self, input: &[f32], output: &mut Vec<u8>) -> Result<(), String>; 
}

pub trait Decoder {
    fn decode(&self, input: &[u8], output: &mut Vec<f32>) -> Result<(), String>;
}

pub struct PCMCodec {
    config: AudioConfig,
}

pub struct OpusCodec {
    config: AudioConfig,
}

impl PCMCodec {
    pub fn new(config: &AudioConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

impl OpusCodec {
    pub fn new(config: &AudioConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

impl Encoder for PCMCodec {
    fn encode(&self, input: &[f32], output: &mut Vec<u8>) -> Result<(), String> {
        for (i, &sample) in input.iter().enumerate() {
            // got this code from claude for the tricky byte manips
            let pre = sample.max(-1.0).min(1.0) * 32767.0;
            let sample_i16: i16 = (pre) as i16;
            if i % 100 == 0 {
                // println!("sample {} -> {}", sample, sample_i16);
            }
            output.write_i16::<byteorder::LittleEndian>(sample_i16).unwrap();
        }
        Ok(())
    }
}

impl Decoder for PCMCodec {
    fn decode(&self, input: &[u8], output: &mut Vec<f32>) -> Result<(), String> {
        // resize output if needed
        let estimated_output_length = input.len() / 2;
        if output.len() != estimated_output_length {
            output.resize(estimated_output_length, 0.0);
        }
        for i2 in 0..input.len() / 2 {
            let i = i2 * 2;
            let sample_i16 = LittleEndian::read_i16(&input[i..i + 2]);
            output[i2] = (sample_i16 as f32 / 32767.0).min(1.0).max(-1.0);
        }
        Ok(())
    }
}

impl Encoder for OpusCodec {
    fn encode(&self, input: &[f32], output: &mut Vec<u8>) -> Result<(), String> {
        todo!("not implemented yet");
    }
}
impl Decoder for OpusCodec {
    fn decode(&self, input: &[u8], output: &mut Vec<f32>) -> Result<(), String> {
        todo!("not implemented yet");
    }
}

pub struct StreamConfig {
    sample_rate: u32,
    frame_size: u32,
    channels: u32,
    codec: Codec,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Codec {
    None,
    Opus
}

impl ToString for Codec {
    fn to_string(&self) -> String {
        match self {
            Codec::None => "none".to_string(),
            Codec::Opus => "opus".to_string(),
        }
    }
}