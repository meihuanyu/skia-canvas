use std::time::{Instant, Duration};
use neon::prelude::*;
use glutin::platform::run_return::EventLoopExtRunReturn;
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::event::{Event, WindowEvent};

use crate::context::{BoxedContext2D};
use crate::utils::{argv, color_arg};

mod view;
use view::View;

mod queue;
use queue::EventQueue;

pub enum CanvasEvent{
  Heartbeat
}

struct Cadence{
  last: Instant,
  wakeup: Duration,
  render: Duration,
}

impl Cadence{
  fn new() -> Self {
    let fps = 60;
    Cadence{
      last: Instant::now(),
      render: Duration::from_nanos(1_000_000_000/fps),
      wakeup: Duration::from_nanos(1_000_000_000/fps * 9/10),
    }
  }

  fn set_frame_rate(&mut self, rate:u64) -> bool{
    let frame_time = 1_000_000_000/rate.max(1);
    let watch_interval = 1_000_000.max(frame_time/10);
    self.render = Duration::from_nanos(frame_time);
    self.wakeup = Duration::from_nanos(frame_time - watch_interval);
    rate > 0
  }

  fn on_next_frame<F:Fn()>(&mut self, draw:F) -> ControlFlow{
    if self.last.elapsed() >= self.render{
      self.last = Instant::now();
      draw();
    }

    match self.last.elapsed() < self.wakeup {
      true => ControlFlow::WaitUntil(self.last + self.wakeup),
      false => ControlFlow::Poll,
    }
  }
}

pub fn display(mut cx: FunctionContext) -> JsResult<JsUndefined> {
  let context = cx.argument::<BoxedContext2D>(0)?;
  let dispatch = cx.argument::<JsFunction>(1)?;
  let animate = cx.argument::<JsFunction>(2)?;
  let matte = color_arg(&mut cx, 3);
  let null = cx.null();

  // display & event handling
  let mut runloop = EventLoop::<CanvasEvent>::with_user_event();
  let mut event_queue = EventQueue::new();
  let mut view = View::new(&runloop, context, matte);

  // runloop state
  let mut cadence = Cadence::new();
  let mut last_move = Instant::now();
  let mut new_loop = true;
  let mut is_stale = true;
  let mut is_fullscreen = false;
  let mut is_animated = false;
  let mut is_done = false;


  let thread_proxy = runloop.create_proxy();
  std::thread::spawn(move || {
    loop{
      std::thread::sleep(std::time::Duration::from_millis(500));
      if thread_proxy.send_event(CanvasEvent::Heartbeat).is_err(){
        break
      }
    }
  });

  // let proxy = runloop.create_proxy();
  runloop.run_return(|event, _, control_flow| {

    if new_loop{
      // starting a new loop after a previous one has exited apparently leaves
      // the control_flow enum still set to Exit
      *control_flow = ControlFlow::Wait;

      // do an initial roundtrip to sync up the Window object's state attrs
      match dispatch.call(&mut cx, null, argv()){
        Ok(result) => {
          let (should_quit, to_fullscreen, to_fps) = view.handle_events(&mut cx, result);
          is_animated = cadence.set_frame_rate(to_fps);
          is_fullscreen = to_fullscreen;
          is_done = should_quit;
          view.go_visible(true);
        },
        Err(_) => is_done = true
      }

      new_loop = false;
    }

    match event {
      Event::NewEvents(..) => {
        if is_done{
          *control_flow = ControlFlow::Exit;
        }else if is_animated{
          *control_flow = cadence.on_next_frame(||
            view.request_redraw()
          );
        }
      }

      Event::UserEvent(CanvasEvent::Heartbeat) => {
        if is_animated && is_fullscreen && last_move.elapsed() > Duration::from_secs(1){
          view.hide_cursor();
        }
      }

      Event::WindowEvent{event, ..} => {
        event_queue.capture(&event, view.dpr());

        match event {
          WindowEvent::CloseRequested => { is_done = true; }
          WindowEvent::CursorMoved{..} => { last_move = Instant::now(); }
          WindowEvent::Resized(physical_size) => {
            // catch fullscreen changes kicked off by window widgets
            if is_fullscreen != view.in_fullscreen() {
              event_queue.went_fullscreen(!is_fullscreen);
              is_fullscreen = !is_fullscreen;
            }
            view.resize(physical_size);
          }
          _ => {  }
        }
      }

      Event::MainEventsCleared => {
        if !event_queue.is_empty(){

          // dispatch UI event-related state changes
          let changes = event_queue.digest(&mut cx);
          match dispatch.call(&mut cx, null, changes){
            Ok(result) => {
              let (should_quit, to_fullscreen, to_fps) = view.handle_events(&mut cx, result);
              if to_fullscreen != is_fullscreen{
                event_queue.went_fullscreen(to_fullscreen);
              }

              is_animated = cadence.set_frame_rate(to_fps);
              is_fullscreen = to_fullscreen;
              is_done = should_quit;
            },
            Err(_) => is_done = true
          }
        }
      }

      Event::RedrawRequested(..) => {
        view.redraw();
        is_stale = true;
      }

      Event::RedrawEventsCleared => {
        if is_stale && is_animated{
          is_stale = false;

          // call the `frame` event handler
          match animate.call(&mut cx, null, argv()){
            Ok(result) => {
              let (should_quit, to_fps) = view.animate(&mut cx, result);
              is_animated = cadence.set_frame_rate(to_fps);
              is_done = should_quit;
            },
            Err(_) => is_done = true
          }
        }
      },

      _ => { }
    }
  });

  Ok(cx.undefined())
}
