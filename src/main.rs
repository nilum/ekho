use warp::Filter;
use cpal::traits::{DeviceTrait, HostTrait, EventLoopTrait};
use cpal::{StreamData, UnknownTypeOutputBuffer};

use std::thread;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::sync::RwLock;

use sample::{signal, Frame, Sample, Signal, ToFrameSliceMut};

const FRAMES_PER_BUFFER: u32 = 512;
const NUM_CHANNELS: i32 = 1;
const SAMPLE_RATE: f64 = 48_000.0;

#[tokio::main]
async fn main() {

    // Set up sound.
    let host = cpal::default_host();
    let event_loop = host.event_loop();

    let device = host.default_output_device().expect("no output device available");

    let mut supported_formats_range = device.supported_output_formats()
        .expect("error while querying formats");
    let format = supported_formats_range.next()
        .expect("no supported format!")
        .with_max_sample_rate();

    let sample_rate = format.sample_rate.0 as f32;
    let mut sample_clock = 0f32;


    
    // Set up beep
    let mut next_value = move |freq: f32| {
        sample_clock = (sample_clock + 1.0) % sample_rate;
        (sample_clock * freq * 2.0 * 3.1415926535897932384626433832795028841971 / sample_rate).sin()
    };

    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    event_loop.play_stream(stream_id).expect("failed to play stream");

    let enabled = Arc::new(RwLock::new(true));
    let freq = Arc::new(RwLock::new(12000.));

    let enabled_lock = enabled.clone();
    let freq_lock = freq.clone();
    
    thread::spawn(move || { event_loop.run(move |stream_id, stream_result| {

        // update with new data from server
        let enabled = *enabled_lock.read().unwrap();
        let freq = *freq_lock.read().unwrap();

        // deal with stream events and write stream data here
        let stream_data = match stream_result {
            Ok(data) => data,
            Err(err) => {
                eprintln!("an error occurred on stream {:?}: {}", stream_id, err);
                return;
            }

        };

        match stream_data {
            StreamData::Output { buffer: UnknownTypeOutputBuffer::F32(mut buffer) } => {

                let mut hz = signal::rate(SAMPLE_RATE).const_hz(freq).sine();
                
                for sample in buffer.chunks_mut(format.channels as usize) {
                    if enabled {
                        //let value = next_value(freq);
                        let value = <f32>::from_sample(hz.next()[0]);
                        for out in sample.iter_mut() {
                            *out = value;
                        }
                    } else {
                        for out in sample.iter_mut() {
                            *out = 0.;
                        }
                    }
                    
                }
            },

            _ => (),
        }
    });
    });

    let tone_e = enabled.clone();
    let tone_f = freq.clone();
    
    let tone_route = warp::path!(u32 / f64)
        .map(move |_enabled, _freq| {
            *tone_e.write().unwrap() = _enabled != 0;
            *tone_f.write().unwrap() = _freq;
            format!("{},{}", _enabled, _freq)
        });

    let index = warp::path("index")
        .and(warp::fs::dir("www/"));

    let status_e = enabled.clone();
    let status_f = freq.clone();
    
    let status = warp::path!("status")
        .map(move || {
            format!("{},{}", *status_e.read().unwrap(), *status_f.read().unwrap())
        });
    
    let routes = index.or(tone_route).or(status);
    
    warp::serve(routes)
        .run(([192, 168, 0, 12], 3030))
        .await;
}
