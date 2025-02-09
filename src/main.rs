use std::process::ExitCode;
use std::io::BufReader;
use std::fs::File;
use std::time::Duration;
use std::{env, thread};

pub mod data;

pub mod tank;
pub mod matchmaker;

use matchmaker::Matchmaker;
use tank::{TankRegistry, TankClass};


fn main() -> ExitCode {

    let args = env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("usage: {} <tomato-tank-perf-30d-json>", &args[0]);
        return ExitCode::SUCCESS;
    }

    let data_reader = File::open(&args[1])
        .map(BufReader::new)
        .expect("open tomato tank perf");

    let data = serde_json::from_reader::<_, Vec<data::tomato::TankPerf>>(data_reader)
        .expect("parse tomato tank perf");

    // Construct the tanks registry from input data.
    let mut registry = TankRegistry::new();
    for perf in &data {
        let tank_class = match perf.class {
            data::tomato::TankClass::Light => TankClass::Light,
            data::tomato::TankClass::Medium => TankClass::Medium,
            data::tomato::TankClass::Heavy => TankClass::Heavy,
            data::tomato::TankClass::Destroyer => TankClass::Destroyer,
            data::tomato::TankClass::Clicker => TankClass::Clicker,
        };
        registry.register(&perf.name, perf.tier, tank_class, perf.battles);
    }

    let battles_speedup = 500;
    let battles_per_second = registry.battles() / 30 / 24 / 3600;
    let battles_interval = Duration::from_secs_f32(1.0 / battles_per_second as f32) / battles_speedup;

    // Running in separate thread to simulate the real timings.
    thread::scope(|scope| {

        let (tx, rx) = crossbeam_channel::bounded(10);
        let registry = &registry;

        scope.spawn(move || {

            let normal_team_size = 15;
            let normal_wait_duration = Duration::from_secs(30);

            let mut match_count = 0usize;
            let mut abnormal_team_size_count = 0usize;
            let mut abnormal_wait_duration_count = 0usize;

            println!("== Running");
            let mut matchmaker = Matchmaker::new(10);

            loop {
                
                matchmaker.queue(rx.recv().unwrap());

                while let Some(m) = matchmaker.poll(2, normal_team_size) {

                    match_count += 1;

                    if matchmaker.len() > 800 {
                        println!(" = Queue length: {}", matchmaker.len());
                    }

                    let wait_duration = m.wait_duration() * battles_speedup;
                    let abnormal_team_size = m.team_size() != normal_team_size;
                    let abnormal_wait_duration = wait_duration > normal_wait_duration;

                    if abnormal_team_size {
                        abnormal_team_size_count += 1;
                    }

                    if abnormal_wait_duration {
                        abnormal_wait_duration_count += 1;
                    }

                    // Only print games that are curious.
                    if abnormal_team_size || abnormal_wait_duration {

                        println!(" = Found match ({}):", m.team_size());
                        for tank_index in 0..m.team_size() {
                            let left_tank = m.tank(0, tank_index);
                            let right_tank = m.tank(1, tank_index);
                            println!(" | {:<4}  {:<20} | {:>20}  {:>4} |", 
                                tier_display(left_tank.tank().tier()),
                                left_tank.tank().name(),
                                right_tank.tank().name(),
                                tier_display(right_tank.tank().tier()));
                        }

                        println!(" |");
                        println!(" | Average wait duration: {:>5.2} s", wait_duration.as_secs_f32());
                        println!(" |");
                        println!(" | Match count: {}", match_count);
                        println!(" | Abnormal team size: {:>5.2} %", abnormal_team_size_count as f32 / match_count as f32 * 100.0);
                        println!(" | Abnormal wait duration: {:>5.2} %", abnormal_wait_duration_count as f32 / match_count as f32 * 100.0);

                    }
                    
                }

            }

        });

        scope.spawn(move || {
            for tank in registry.pick_many_random() {
                tx.send(tank).unwrap();
                thread::sleep(battles_interval);
            }
        });

    });

    ExitCode::SUCCESS

}

fn tier_display(tier: u8) -> &'static str {
    match tier {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        5 => "V",
        6 => "VI",
        7 => "VII",
        8 => "VIII",
        9 => "IX",
        10 => "X",
        _ => "?",
    }
}
