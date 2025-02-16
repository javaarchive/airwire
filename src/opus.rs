
use crate::audio::{hexdump_debug, Decoder, Encoder};
use crate::AudioConfig;
use opus::{Application, Channels, Decoder as OpusDecoder, Encoder as OpusEncoder};

pub struct OpusCodec {
    config: AudioConfig,
    encoder: OpusEncoder,
    decoder: OpusDecoder,
}

pub fn parse_channel(channels: u16) -> Channels {
    match channels {
        1 => Channels::Mono,
        2 => Channels::Stereo,
        // tbh you can mod the opus lib for there, this restriction might just be 
        _ => panic!("unsupported channel count {}", channels)
    }
}

pub fn parse_application(profile: &str) -> Application {
    match profile {
        "voip" => Application::Voip,
        "lowdelay" => Application::LowDelay,
        "lowlatency" => Application::LowDelay,
        _ => Application::Audio
    }
}

impl OpusCodec {
    pub fn new(config: &AudioConfig) -> Self {
        let channels = parse_channel(config.channels);
        let mut encoder = OpusEncoder::new(config.sample_rate, channels, parse_application(&config.profile)).expect("opus encoder init failure") ;
        let decoder = OpusDecoder::new(config.sample_rate, channels).expect("opus decoder init failure");

        if config.bitrate == 0 {
            encoder.set_bitrate(opus::Bitrate::Auto).expect("opus bitrate set to auto failure");
        } else if config.bitrate < 0 {
            encoder.set_bitrate(opus::Bitrate::Max).expect("opus bitrate set to max failure");
        } else {
            encoder.set_bitrate(opus::Bitrate::Bits(1024 * config.bitrate)).expect(&format!("opus bitrate set to {}kbps failure", config.bitrate));
        }

        encoder.set_inband_fec(config.fec).expect("opus inband fec set failure");
        encoder.set_vbr(config.vbr).expect("opus vbr set failure");
        // encoder.set_packet_loss_perc(value)

        // TODO: packet loss percentage?

        Self {
            config: config.clone(),
            encoder: encoder,
            decoder: decoder
        }
    }
}

impl Encoder for OpusCodec {
    fn encode(&mut self, input: &[f32], output: &mut Vec<u8>) -> Result<(), String> {
        match self.encoder.encode_float(input, output) {
            Ok(wrote) => {
                output.resize(wrote, 0); // this will only shrink
                // println!("encode {} bytes sample {}", wrote, input[69]);
                // hexdump_debug(output);
                Ok(())
            },
            Err(err) => {
                // Err(format!("opus encoding got an error: {:?}", err))
                Err(format!("opus encoding got an error: {:?} {:?} {}", err, input, input.len()))
            }
        }
    }
}
impl Decoder for OpusCodec {
    fn decode(&mut self, input: &[u8], output: &mut Vec<f32>) -> Result<(), String> {
        // println!("in {} out {}", input.len(), output.len());
        match self.decoder.decode_float(input, output, self.config.fec) {
            Ok(_) => {
                Ok(())
            },
            Err(err) => {
                if self.config.debug {
                    hexdump_debug(input);
                }
                Err(format!("opus decoding got an error: {:?} input: {} output: {}", err, input.len(), output.len()))
            },
        }
    }
}