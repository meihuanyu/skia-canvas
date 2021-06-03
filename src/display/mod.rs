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

use window::Window;
use event::CanvasEvent;

pub struct Cadence{
  rate: u64,
  last: Instant,
  wakeup: Duration,
  render: Duration,
}

impl Cadence{
  pub fn new() -> Self {
    let rate = 60;
    Cadence{
      rate,
      last: Instant::now(),
      render: Duration::from_nanos(1_000_000_000/rate),
      wakeup: Duration::from_nanos(1_000_000_000/rate * 9/10),
    }
  }

  pub fn set_frame_rate(&mut self, rate:u64){
    let frame_time = 1_000_000_000/rate.max(1);
    let watch_interval = 1_000_000.max(frame_time/10);
    self.render = Duration::from_nanos(frame_time);
    self.wakeup = Duration::from_nanos(frame_time - watch_interval);
    self.rate = rate;
  }

  pub fn on_next_frame<F:Fn()>(&mut self, draw:F) -> ControlFlow{
    if self.last.elapsed() >= self.render{
      self.last = Instant::now();
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
  let mut running = false;

  runloop.run_return(|event, _, control_flow| {

    if !running{
      // do an initial roundtrip to sync up the Window object's state attrs
      *control_flow = window.communicate(&mut cx, &dispatch);
      window.show();
      running = true;
    }

    window.handle_event(&event);

    match event {
      Event::NewEvents(..) => {
        if cadence.active() {
          *control_flow = cadence.on_next_frame(|| window.render() );
        }
      }

      Event::UserEvent(canvas_event) => {
        match canvas_event{
          CanvasEvent::Close => *control_flow = ControlFlow::Exit,
          CanvasEvent::FrameRate(fps) => {
            cadence.set_frame_rate(fps);
          }
          _ => view.handle_event(&canvas_event)
        }
      }

      Event::WindowEvent{event, ..} => {
        match event {
          WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
          _ => {}
        }
      }

      Event::MainEventsCleared => {
        // do a dispatch-events round-trip
        *control_flow = window.communicate_pending(&mut cx, &dispatch);
      }

      Event::RedrawRequested(..) => {
        *control_flow = window.communicate(&mut cx, &animate);
      }

      Event::RedrawEventsCleared => {}

      _ => {}
    }

  });

  Ok(cx.undefined())
}
