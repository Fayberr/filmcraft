//! Regression test for #431: the monitor toolbar's '+' is a working Button Editor.
//! Driven through the real egui UI and its automation controls, without a GPU.

use std::sync::mpsc::{Sender, channel};

use egui_kittest::Harness;
use filmcraft_engine::Session;
use filmcraft_ui_egui::FilmcraftApp;
use filmcraft_ui_egui::control::ControlRequest;
use serde_json::{Value, json};

struct Driver {
    harness: Harness<'static, FilmcraftApp>,
    tx: Sender<ControlRequest>,
}

impl Driver {
    fn demo() -> Self {
        let mut s = Session::default();
        s.execute("file.openDemoProject", json!({})).unwrap();
        let (tx, rx) = channel();
        let app = FilmcraftApp::new(s).with_control(rx);
        let harness = Harness::builder().with_size(egui::vec2(1600.0, 980.0)).with_max_steps(10_000).build_eframe(move |_cc| app);
        let mut d = Self { harness, tx };
        d.frames(4);
        d
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            let ctx = self.harness.ctx.clone();
            let mut raw = std::mem::take(self.harness.input_mut());
            eframe::App::raw_input_hook(self.harness.state_mut(), &ctx, &mut raw);
            *self.harness.input_mut() = raw;
            self.harness.step();
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Value {
        let (req, reply) = ControlRequest::new(method, params.clone());
        self.tx.send(req).unwrap();
        for _ in 0..600 {
            self.frames(1);
            if let Ok(v) = reply.try_recv() {
                assert_eq!(v["ok"], json!(true), "{method} {params}: {v}");
                return v["result"].clone();
            }
        }
        panic!("no reply to {method} {params}");
    }

    fn ids(&mut self, prefix: &str) -> Vec<String> {
        let v = self.call("ui.elements", json!({"prefix": prefix}));
        v.as_array().unwrap().iter().filter_map(|e| e["id"].as_str().map(str::to_string)).collect()
    }

    fn has(&mut self, id: &str) -> bool {
        self.ids(id).iter().any(|s| s == id)
    }

    fn click(&mut self, id: &str) {
        self.call("ui.click", json!({"id": id}));
        self.frames(3);
    }

    fn open_editor(&mut self) {
        if !self.has("program.buttonEditor.reset") {
            self.click("program.buttonEditor");
        }
        assert!(self.has("program.buttonEditor.reset"), "the '+' opens the editor");
    }
}

#[test]
fn program_monitor_button_editor_can_show_hide_and_reset_controls() {
    let mut d = Driver::demo();
    assert!(d.has("program.buttonEditor"));
    assert!(d.has("program.transport.markers.add"), "existing buttons are shown by default");
    assert!(!d.has("program.transport.playback.loop"), "Loop begins hidden");

    d.open_editor();
    d.click("program.buttonEditor.playback.loop");
    assert!(d.harness.state().ui.program.show_loop_button);
    assert!(d.has("program.transport.playback.loop"), "enabling Loop puts it in the toolbar");

    d.open_editor();
    d.click("program.buttonEditor.markers.add");
    assert!(!d.has("program.transport.markers.add"), "disabled control is removed from toolbar");

    d.open_editor();
    d.click("program.buttonEditor.reset");
    assert!(d.has("program.transport.markers.add"), "Reset restores original buttons");
    assert!(!d.has("program.transport.playback.loop"), "Reset hides optional Loop again");
}
