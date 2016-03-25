extern crate amethyst;

use amethyst::engine::{Application, Duration, State, Trans};

struct HelloWorld;

impl State for HelloWorld {
    fn on_start(&mut self) {
        println!("Game started!");
    }

    fn update(&mut self, _delta: Duration) -> Trans {
        println!("Hello from Amethyst!");
        Trans::Quit
    }

    fn on_stop(&mut self) {
        println!("Game stopped!");
    }
}

fn main() {
    let mut game = Application::new(HelloWorld);
    game.run();
}
