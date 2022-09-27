#[macro_use]
extern crate lazy_static;
extern crate rusttype;
extern crate dbsdk_rs;
use std::{sync::RwLock};

use dbsdk_rs::{vdp::{self}, io::{FileStream, FileMode}, db};
use gamefont::GameFont;
use lazy_static::initialize;

mod gamefont;

struct MyApp<'a> {
    font: GameFont<'a>,
}

impl<'a> MyApp<'a> {
    pub fn new() -> MyApp<'a> {
        let mut font_file = FileStream::open("/cd/content/Montserrat-Medium.ttf", FileMode::Read).expect("Failed opening font file");
        let gamefont = GameFont::new(&mut font_file, 512);

        MyApp { font: gamefont }
    }

    fn tick(&mut self) {
        vdp::clear_color(vdp::Color32::new(128, 128, 255, 255));
        vdp::clear_depth(1.0);

        vdp::depth_write(false);
        vdp::depth_func(vdp::Compare::Always);
        
        self.font.draw_text(8, 8, 24, "Hello, world!");
    }
}

lazy_static! {
    static ref MY_APP: RwLock<MyApp<'static>> = RwLock::new(MyApp::new());
}

fn tick() {
    let mut my_app = MY_APP.write().unwrap();
    my_app.tick();
}

#[no_mangle]
pub fn main(_: i32, _: i32) -> i32 {
    db::register_panic();
    initialize(&MY_APP);
    vdp::set_vsync_handler(Some(tick));
    return 0;
}