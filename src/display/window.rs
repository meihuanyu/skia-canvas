#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use std::time::{Instant, Duration};
use neon::prelude::*;
use neon::result::Throw;
use skia_safe::{Color};
use glutin::platform::run_return::EventLoopExtRunReturn;
use glutin::event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopClosed};
use glutin::event::{Event, WindowEvent};
use glutin::dpi::{LogicalSize, PhysicalSize, LogicalPosition};
use glutin::window::CursorIcon;

use crate::utils::to_cursor_icon;
use crate::canvas::Page;
use crate::context::BoxedContext2D;
use crate::utils::{argv, color_arg, float_arg};
use super::{CanvasEvent};
use super::view::View;
use super::event;

pub struct Window{
  proxy: EventLoopProxy<CanvasEvent>,
  position: LogicalPosition<i32>,
  size: LogicalSize<u32>,
  events: event::Sieve,

  title: String,
  cursor: Option<CursorIcon>,
  dpr: f64,

  animated: bool,
  fps: u64,

  closed: bool,
  fullscreen: bool,
  needs_display: bool,
}


impl Window{
  pub fn new(runloop:&EventLoop<CanvasEvent>, width:f32, height:f32) -> Self {
    Window{
      proxy: runloop.create_proxy(),
      position: LogicalPosition::new(0,0),
      size: LogicalSize::new(width as u32, height as u32),
      events: event::Sieve::new(),

      title: "".to_string(),
      cursor: Some(CursorIcon::Default),
      dpr: 1.0,
      fps: 0,

      closed: false,
      animated: false,
      fullscreen: false,
      needs_display:false,
    }
  }

  pub fn new_view(&mut self, runloop:&EventLoop<CanvasEvent>, c2d:Handle<BoxedContext2D>, backdrop:Option<Color>) -> View {
    let mut view = View::new(&runloop, c2d, backdrop, self.size.width as f32, self.size.height as f32);
    self.dpr = view.dpr();
    view
  }

  pub fn show(&self){
    self.proxy.send_event(CanvasEvent::Visible(true)).ok();
  }

  pub fn render(&self){
    self.proxy.send_event(CanvasEvent::Render).ok();
  }


  pub fn handle_event(&mut self, event:&Event<CanvasEvent>){
    if let Event::WindowEvent{event, ..} = event {
      if let WindowEvent::Resized(physical_size) = event {
        self.size = LogicalSize::from_physical(*physical_size, self.dpr);
      }

      if let WindowEvent::Moved(physical_pt) = event {
        self.position = LogicalPosition::from_physical(*physical_pt, self.dpr);
      }

      self.events.capture(&event, self.dpr)
    }
  }

  pub fn communicate_pending(&mut self, cx: &mut FunctionContext, callback:&Handle<JsFunction>) -> ControlFlow {
    match self.events.is_empty(){
      true => ControlFlow::Poll,
      false => self.communicate(cx, callback)
    }
  }

  pub fn communicate(&mut self, cx: &mut FunctionContext, callback:&Handle<JsFunction>) -> ControlFlow {
    let gui_events = self.events.serialized(cx);
    let null = cx.null();
    if let Ok(response) = callback.call(cx, null, gui_events){
      if self.handle_feedback(cx, response).is_ok(){
        return ControlFlow::Poll
      }
    }
    ControlFlow::Exit
  }

  pub fn handle_feedback(&mut self, cx:&mut FunctionContext, feedback:Handle<JsValue>) -> Result<(), EventLoopClosed<CanvasEvent>> {
    if let Ok(array) = feedback.downcast::<JsArray, _>(cx){
      if let Ok(vals) = array.to_vec(cx){

        // 0: context
        if let Ok(c2d) = vals[0].downcast::<BoxedContext2D, _>(cx){
          let page = c2d.borrow_mut().get_page();
          self.proxy.send_event(CanvasEvent::Page(page))?
        }

        // 1: title
        if let Ok(title) = vals[1].downcast::<JsString, _>(cx){
          let title = title.value(cx);
          if title != self.title{
            self.title = title.to_string();
            self.proxy.send_event(CanvasEvent::Title(title))?
          }
        }

        // 2: 'keep running' flag
        if let Ok(active) = vals[2].downcast::<JsBoolean, _>(cx){
          if !active.value(cx){
            self.proxy.send_event(CanvasEvent::Close)?
          }
        }

        // 3: fullscreen flag
        if let Ok(is_full) = vals[3].downcast::<JsBoolean, _>(cx){
          let is_full = is_full.value(cx);
          // self.proxy.send_event(CanvasEvent::Fullscreen(is_full));
        }

        // 4: fps (or zero to disable animation)
        if let Ok(fps) = vals[4].downcast::<JsNumber, _>(cx){
          let fps = fps.value(cx) as u64;
          if fps != self.fps{
            self.fps = fps;
            self.proxy.send_event(CanvasEvent::FrameRate(fps))?
          }
        }

        // 5+6: window size
        if let Ok(width) = vals[5].downcast::<JsNumber, _>(cx){
          if let Ok(height) = vals[6].downcast::<JsNumber, _>(cx){
            let size = LogicalSize::new( width.value(cx) as u32, height.value(cx) as u32 );
            if size != self.size{
              self.size = size;
              self.proxy.send_event(CanvasEvent::Size(size))?
            }
          }
        }

        // 7+8: window position
        if let Ok(x) = vals[7].downcast::<JsNumber, _>(cx){
          if let Ok(y) = vals[8].downcast::<JsNumber, _>(cx){
            let position = LogicalPosition::new( x.value(cx) as i32, y.value(cx) as i32 );
            if position != self.position{
              self.position = position;
              self.proxy.send_event(CanvasEvent::Position(position))?
            }
          }
        }

        // 9: cursor
        if let Ok(cursor_style) = vals[9].downcast::<JsString, _>(cx){
          let cursor_style = cursor_style.value(cx);
          let cursor_icon = to_cursor_icon(&cursor_style);
          if cursor_icon != self.cursor{
            self.cursor = cursor_icon;
            if cursor_icon.is_some() || cursor_style == "none"{
              self.proxy.send_event(CanvasEvent::Cursor(cursor_icon))?
            }
          }
        }
      }
    }

    Ok(())
  }

}