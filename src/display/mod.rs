#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(clippy::single_match)]
#![allow(clippy::collapsible_match)]
use std::time::{Instant, Duration};
use neon::prelude::*;
use glutin::platform::run_return::EventLoopExtRunReturn;
use glutin::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use glutin::event::{Event, WindowEvent};
use glutin::dpi::{LogicalSize, PhysicalSize, LogicalPosition};
use glutin::window::CursorIcon;

use crate::canvas::Page;
use crate::context::BoxedContext2D;
use crate::utils::{argv, color_arg, float_arg};

mod window;
mod event;
mod view;

pub use window::Window;
pub use event::CanvasEvent;
pub use view::{View, Fit};

pub struct Cadence{
  rate: u64,
  last: Instant,
  wakeup: Duration,
  render: Duration,
  begun: bool,
}

impl Cadence{
  pub fn new() -> Self {
    Cadence{
      rate: 0,
      last: Instant::now(),
      render: Duration::new(0, 0),
      wakeup: Duration::new(0, 0),
      begun: false,
    }
  }

  fn on_startup<F:FnOnce()>(&mut self, init:F){
    if self.begun{ return }
    self.begun = true;
    init();
  }

  pub fn set_frame_rate(&mut self, rate:u64){
    let frame_time = 1_000_000_000/rate.max(1);
    let watch_interval = 1_000_000.max(frame_time/10);
    self.render = Duration::from_nanos(frame_time);
    self.wakeup = Duration::from_nanos(frame_time - watch_interval);
    self.rate = rate;
  }

  pub fn on_next_frame<F:Fn()>(&mut self, draw:F) -> ControlFlow{
    if self.rate == 0{
      return ControlFlow::Wait;
    }

    if self.last.elapsed() >= self.render{
      while self.last < Instant::now() - self.render{
        self.last += self.render
      }
      draw();
    }

    match self.last.elapsed() < self.wakeup {
      true => ControlFlow::WaitUntil(self.last + self.wakeup),
      false => ControlFlow::Poll,
    }
  }

  pub fn active(&self) -> bool{
    self.rate > 0
  }
}

pub fn begin(mut cx: FunctionContext) -> JsResult<JsUndefined> {
  let context = cx.argument::<BoxedContext2D>(0)?;
  let dispatch = cx.argument::<JsFunction>(1)?;
  let animate = cx.argument::<JsFunction>(2)?;
  let matte = color_arg(&mut cx, 3);
  let width = float_arg(&mut cx, 4, "width")?;
  let height = float_arg(&mut cx, 5, "height")?;
  let null = cx.null();

  // display & event handling
  let mut runloop = EventLoop::<CanvasEvent>::with_user_event();
  let mut window = Window::new(&runloop, width, height);
  let mut view = window.new_view(&runloop, context, matte);
  let mut cadence = Cadence::new();
  let mut halt = false;

  let proxy = runloop.create_proxy();
  std::thread::spawn(move || loop {
    std::thread::sleep(Duration::from_millis(500));
    if proxy.send_event(CanvasEvent::Heartbeat).is_err(){ break }
  });

  runloop.run_return(|event, _, control_flow| {

    cadence.on_startup(||{
      // do an initial roundtrip to sync up the Window object's state attrs
      halt = window.communicate(&mut cx, &dispatch).is_err();
      *control_flow = ControlFlow::Wait;
    });

    match event {
      Event::NewEvents(..) => {
        *control_flow = cadence.on_next_frame(|| window.render() );
      }

      Event::WindowEvent{event, ..} => {
        window.handle_ui_event(&event);
      }

      Event::UserEvent(canvas_event) => {
        match canvas_event{
          CanvasEvent::Close => halt = true,
          CanvasEvent::Heartbeat => window.autohide_cursor(),
          CanvasEvent::FrameRate(fps) => cadence.set_frame_rate(fps),
          CanvasEvent::InFullscreen(to_full) => window.went_fullscreen(to_full),
          CanvasEvent::Transform(matrix) => window.new_transform(matrix),
          _ => window.send_js_event(canvas_event)
        }
      }

      Event::MainEventsCleared => {
        view.handle_js_events();
        halt = window.communicate_pending(&mut cx, &dispatch).is_err();
      }

      Event::RedrawRequested(..) => {
        if cadence.active(){
          halt = window.communicate(&mut cx, &animate).is_err()
        }
      }

      _ => {}
    }

    if halt{ *control_flow = ControlFlow::Exit; }
  });

  Ok(cx.undefined())
}
