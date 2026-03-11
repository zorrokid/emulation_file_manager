use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::error::LibretroError;

/// Owns the cpal audio output stream and the sample buffer shared with the
/// libretro audio callbacks.
///
/// The stream runs on a private cpal audio thread. The libretro callbacks
/// (which run on the GTK main thread inside retro_run()) push f32 samples
/// into the shared buffer; cpal drains it on the audio thread.
///
/// Dropping this struct stops the stream (cpal::Stream's Drop impl does this).
pub struct AudioOutput {
    // Keep-alive: cpal stops the stream when Stream is dropped.
    _stream: cpal::Stream,
}

impl AudioOutput {
    /// Open the default audio output device and start a stream at `sample_rate` Hz.
    ///
    /// `sample_buffer` is the shared ring buffer that the libretro audio callbacks
    /// push into. We accept it as a parameter (rather than creating it here) so
    /// that the buffer can be installed into `CoreCallbackState` before the core
    /// is initialised — we only know the real sample rate after `retro_load_game`.
    ///
    /// Returns `LibretroError::AudioInit` if no output device is available or
    /// the device refuses the requested configuration.
    pub fn new(
        sample_rate: u32,
        sample_buffer: Arc<Mutex<VecDeque<f32>>>,
    ) -> Result<Self, LibretroError> {
        // cpal's Host is the platform audio API (ALSA / PulseAudio on Linux).
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or_else(|| LibretroError::AudioInit("No audio output device found".into()))?;

        // Request stereo f32 output at the core's native sample rate.
        // Most sound cards and PulseAudio support this directly; if not,
        // cpal will return a BuildStreamError and we log a warning and
        // continue without audio rather than crashing.
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(sample_rate),
            // Let the driver choose a buffer size — avoids both underruns
            // (too small) and latency (too large).
            buffer_size: cpal::BufferSize::Default,
        };

        // Clone the Arc for the cpal closure — cpal requires 'static closures
        // because the audio thread outlives the call to build_output_stream.
        let buffer_for_stream = Arc::clone(&sample_buffer);

        // build_output_stream calls `data_callback` each time the driver needs
        // more audio. We fill `data` from our ring buffer; silence (0.0) if
        // we're momentarily empty (underrun).
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    let mut buf = buffer_for_stream.lock().expect("audio buffer lock");
                    for sample in data.iter_mut() {
                        // pop_front returns None on underrun → output silence.
                        *sample = buf.pop_front().unwrap_or(0.0);
                    }
                },
                // Error callback — log and continue; non-fatal.
                |err| tracing::warn!("cpal audio stream error: {err}"),
                None, // no timeout
            )
            .map_err(|e| LibretroError::AudioInit(format!("Failed to build audio stream: {e}")))?;

        // Start playback. The cpal thread begins calling data_callback immediately.
        stream
            .play()
            .map_err(|e| LibretroError::AudioInit(format!("Failed to start audio stream: {e}")))?;

        Ok(Self { _stream: stream })
    }
}
