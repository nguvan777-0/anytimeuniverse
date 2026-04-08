use std::sync::{Arc, RwLock};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// A non-blocking bridge to stream raw float metrics from the determinism engine to the 44.1k audio thread.
pub struct SynthEngine {
    params: Arc<RwLock<[f32; 15]>>,
    volume: Arc<RwLock<f32>>,
    _stream: Option<cpal::Stream>, // Keep the background stream alive
}

impl Default for SynthEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthEngine {
    pub fn new() -> Self {
        let params = Arc::new(RwLock::new([0.0; 15]));
        let volume = Arc::new(RwLock::new(0.0));
        
        let params_clone = Arc::clone(&params);
        let vol_clone = Arc::clone(&volume);
        
        let host = cpal::default_host();
        let stream = if let Some(device) = host.default_output_device() {
            if let Ok(config) = device.default_output_config() {
                let sample_rate = config.sample_rate().0 as f32;
                let channels = config.channels() as usize;

                let mut phase = [0.0f32; 3];
                let mut current_params = [0.0f32; 15];
                let mut current_vol = 0.0f32;

                let stream_result = device.build_output_stream(
                    &config.into(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // Non-blocking try_read so the GUI thread isn't blocked by audio interrupts
                        if let Ok(p) = params_clone.try_read() { current_params = *p; }
                        if let Ok(v) = vol_clone.try_read() { current_vol = *v; }

                        let mut index = 0;
                        while index < data.len() {
                            let mut sample = 0.0f32;
                            
                            // 3-Oscillator Additive Synthesizer (One for each Branch)
                            for w in 0..3 {
                                let base = w * 5;
                                let amp = current_params[base].abs();       
                                let freq = current_params[base + 1].abs();  
                                let _angle = current_params[base + 2].abs();
                                let shape = current_params[base + 3].abs(); 
                                let warp = current_params[base + 4].abs();

                                // Pitch Math: Mapped from ~5.0 params speed to ~100-300Hz drone
                                let hz = 50.0 + (freq * 50.0);
                                phase[w] = (phase[w] + hz / sample_rate) % 1.0;
                                
                                let rad = phase[w] * std::f32::consts::TAU;
                                // Core Osc: Pure Sine
                                let sine = rad.sin();
                                // Mutated Osc: Sawtooth for harsh/faceted paramss
                                let saw = (phase[w] * 2.0) - 1.0;
                                
                                // Timbre interpolates purely based on branch geometry (shape)
                                let timbre_mix = (shape * 0.2).clamp(0.0, 1.0);
                                let oscillator = sine * (1.0 - timbre_mix) + saw * timbre_mix;
                                
                                // Vibrato depth driven by Warp parameter
                                let vibrato = 1.0 + (phase[w] * warp * 10.0).sin() * 0.05;

                                sample += oscillator * vibrato * amp * 0.2 * current_vol;
                            }
                            
                            sample = sample.clamp(-1.0, 1.0);

                            for _ in 0..channels {
                                data[index] = sample;
                                index += 1;
                            }
                        }
                    },
                    |err| eprintln!("Audio stream error: {}", err),
                    None
                );
                
                if let Ok(s) = stream_result {
                    let _ = s.play();
                    Some(s)
                } else {
                    None
                }
            } else { None }
        } else { None };

        Self {
            params,
            volume,
            _stream: stream,
        }
    }

    pub fn set_params(&self, new_params: [f32; 15]) {
        // High-frequency injection from the 60fps GUI thread
        if let Ok(mut p) = self.params.try_write() {
            *p = new_params;
        }
    }

    pub fn set_volume(&self, vol: f32) {
        if let Ok(mut v) = self.volume.try_write() {
            *v = vol;
        }
    }
}
