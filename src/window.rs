#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::time::{Instant, Duration};
use neon::prelude::*;

use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::{BackendRenderTarget, SurfaceOrigin, DirectContext};
use skia_safe::{Rect, Color, ColorType, Surface, Picture};

use glutin::platform::run_return::EventLoopExtRunReturn;
use glutin::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent, ModifiersState, ElementState, MouseButton, MouseScrollDelta};
use glutin::dpi::{LogicalSize, PhysicalSize, LogicalPosition, PhysicalPosition};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::{WindowBuilder, Fullscreen};
use glutin::GlProfile;
use gl::types::*;

use crate::context::{BoxedContext2D};
use crate::utils::*;

type WindowedContext = glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>;

struct View{
  ident: (usize, usize),
  pict: Picture,
  dims: (f32, f32),
  title: String,
  context: WindowedContext,
  surface: RefCell<Surface>,
  gl: RefCell<DirectContext>,
  backdrop: Color
}

impl View{
  fn new(runloop:&EventLoop<()>, c2d:Handle<BoxedContext2D>, backdrop:Color) -> Self{
    let wb = WindowBuilder::new()
      .with_transparent(backdrop.a() < 255)
      .with_min_inner_size(LogicalSize::new(75,75))
      .with_visible(false)
      .with_title("");

    let cb = glutin::ContextBuilder::new()
      .with_depth_buffer(0)
      .with_stencil_buffer(8)
      .with_pixel_format(24, 8)
      .with_double_buffer(Some(true))
      .with_gl_profile(GlProfile::Core);

    let context = cb.build_windowed(wb, &runloop).unwrap();
    let context = unsafe { context.make_current().unwrap() };
    gl::load_with(|s| context.get_proc_address(&s));

    let mut ctx = c2d.borrow_mut();
    let bounds = ctx.bounds;
    let (width, height) = (bounds.width(), bounds.height());
    let size = LogicalSize::new(width, height);

    context.window().set_inner_size(size);
    if let Some(monitor) = context.window().current_monitor(){
      let screen_size = LogicalSize::<f32>::from_physical(monitor.size(), monitor.scale_factor());
      let position = LogicalPosition::new(
        (screen_size.width - size.width) / 2.0,
        (screen_size.height - size.height) / 3.0,
      );
      context.window().set_outer_position(position);
    }

    let (gl, surface) = View::gl_surface(&context);
    View{
      context,
      ident: ctx.ident(),
      title: "".to_string(),
      pict: ctx.get_picture(None).unwrap(),
      dims: (width, height),
      surface: RefCell::new(surface),
      gl: RefCell::new(gl),
      backdrop
    }
  }

  fn gl_surface(windowed_context: &WindowedContext) -> (DirectContext, Surface) {
    let mut gl_context = DirectContext::new_gl(None, None).unwrap();

    let fb_info = {
      let mut fboid: GLint = 0;
      unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

      FramebufferInfo {
        fboid: fboid.try_into().unwrap(),
        format: skia_safe::gpu::gl::Format::RGBA8.into(),
      }
    };

    let pixel_format = windowed_context.get_pixel_format();
    let size = windowed_context.window().inner_size();
    let backend_render_target = BackendRenderTarget::new_gl(
      (
        size.width.try_into().unwrap(),
        size.height.try_into().unwrap(),
      ),
      pixel_format.multisampling.map(|s| s.try_into().unwrap()),
      pixel_format.stencil_bits.try_into().unwrap(),
      fb_info,
    );

    let surface = Surface::from_backend_render_target(
      &mut gl_context,
      &backend_render_target,
      SurfaceOrigin::BottomLeft,
      ColorType::RGBA8888,
      None,
      None,
    );

    (gl_context, surface.unwrap())
  }

  fn dpr(&self) -> f64{
    self.context.window().scale_factor() as f64
  }

  fn resize(&self, physical_size:PhysicalSize<u32>){
    let (gl, surface) = View::gl_surface(&self.context);
    self.context.resize(physical_size);
    self.surface.replace(surface);
    self.gl.replace(gl);
  }

  fn in_fullscreen(&self) -> bool {
    self.context.window().fullscreen().is_some()
  }

  fn go_fullscreen(&mut self, to_full:bool){
    let mode = match to_full{
      true => Some(Fullscreen::Borderless(None)),
      false => None
    };

    self.context.window().set_fullscreen(mode);
  }

  fn redraw(&self){
    let mut surface = self.surface.borrow_mut();
    let canvas = surface.canvas();

    let physical_size = self.context.window().inner_size();
    let sf = physical_size.height as f32 / self.dims.1;
    let indent = (physical_size.width as f32 - self.dims.0 * sf) / 2.0;
    let clip = Rect::from_size(self.dims);

    canvas.clear(self.backdrop);
    canvas.save();
    canvas.translate((indent, 0.0)).scale((sf, sf));
    canvas.clip_rect(clip, None, None);
    canvas.draw_picture(&self.pict, None, None);
    canvas.restore();

    let mut gl = self.gl.borrow_mut();
    gl.flush(None);
    self.context.swap_buffers().unwrap();
  }

  fn animate(&mut self, cx:&mut FunctionContext, result:Handle<JsValue>) -> (bool, u64){
    let mut should_quit = false;
    let mut to_fps = 0;

    if let Ok(array) = result.downcast::<JsArray, _>(cx){
      if let Ok(vals) = array.to_vec(cx){

        if let Ok(c2d) = vals[0].downcast::<BoxedContext2D, _>(cx){
          let mut ctx = c2d.borrow_mut();
          if self.ident != ctx.ident(){
            let pict = ctx.get_picture(None).unwrap();
            let bounds = ctx.bounds;
            self.pict = pict;
            self.dims = (bounds.width(), bounds.height());
            self.ident = ctx.ident();
          }
        }

        if let Ok(active) = vals[1].downcast::<JsBoolean, _>(cx){
          if !active.value(cx){ should_quit = true }
        }

        if let Ok(fps) = vals[2].downcast::<JsNumber, _>(cx){
          to_fps = fps.value(cx) as u64;
        }

      }
    }
    (should_quit, to_fps)
  }

  fn handle_events(&mut self, cx:&mut FunctionContext, result:Handle<JsValue>) -> (bool, bool, u64){
    let mut window = self.context.window();
    let mut should_quit = false;
    let mut to_fullscreen = false;
    let mut to_fps = 0;

    if let Ok(array) = result.downcast::<JsArray, _>(cx){
      if let Ok(vals) = array.to_vec(cx){

        // 0: context
        if let Ok(c2d) = vals[0].downcast::<BoxedContext2D, _>(cx){
          let mut ctx = c2d.borrow_mut();
          if self.ident != ctx.ident(){
            let pict = ctx.get_picture(None).unwrap();
            let bounds = ctx.bounds;
            self.pict = pict;
            self.dims = (bounds.width(), bounds.height());
            self.ident = ctx.ident();
          }
        }

        // 1: title
        if let Ok(title) = vals[1].downcast::<JsString, _>(cx){
          let title = title.value(cx);
          if self.title != title{
            self.title = title;
            window.set_title(&self.title);
          }
        }

        // 2: 'keep running' flag
        if let Ok(active) = vals[2].downcast::<JsBoolean, _>(cx){
          if !active.value(cx){ should_quit = true }
        }

        // 3: fullscreen flag
        if let Ok(is_full) = vals[3].downcast::<JsBoolean, _>(cx){
          let is_full = is_full.value(cx);
          let was_full = window.fullscreen().is_some();
          if is_full != was_full{
            match is_full{
              true => window.set_fullscreen( Some(Fullscreen::Borderless(None)) ),
              false => window.set_fullscreen( None )
            }
          }
          to_fullscreen = is_full
        }

        // 4: fps (or zero to disable animation)
        if let Ok(fps) = vals[4].downcast::<JsNumber, _>(cx){
          to_fps = fps.value(cx) as u64;
        }

        // 5+6: window size
        let dpr = self.dpr();
        let old_dims = window.inner_size();
        let old_dims = LogicalSize::from_physical(old_dims, dpr);
        let mut new_dims = old_dims;
        if let Ok(width) = vals[5].downcast::<JsNumber, _>(cx){
          new_dims.width = width.value(cx) as i32;
        }
        if let Ok(height) = vals[6].downcast::<JsNumber, _>(cx){
          new_dims.height = height.value(cx) as i32;
        }
        if new_dims != old_dims{
          window.set_inner_size(new_dims);
        }

        // 7+8: window position
        let old_pos = window.outer_position().unwrap();
        let old_pos = LogicalPosition::from_physical(old_pos, dpr);
        let mut new_pos = old_pos;
        if let Ok(x) = vals[7].downcast::<JsNumber, _>(cx){
          new_pos.x = x.value(cx) as i32;
        }
        if let Ok(y) = vals[8].downcast::<JsNumber, _>(cx){
          new_pos.y = y.value(cx) as i32;
        }
        if new_pos != old_pos{
          window.set_outer_position(new_pos);
        }

        // 9: cursor
        if let Ok(cursor_style) = vals[9].downcast::<JsString, _>(cx){
          let cursor_style = cursor_style.value(cx);
          match to_cursor_icon(&cursor_style){
            Some(icon) => {
              window.set_cursor_icon(icon);
              window.set_cursor_visible(true);
            },
            None => {
              if cursor_style == "none" {
                window.set_cursor_visible(false);
              }
            }
          }
        }

      }
    }

    (should_quit, to_fullscreen, to_fps)
  }
}

struct Cadence{
  last: Instant,
  render: Duration,
  wakeup: Duration,
}

impl Cadence{
  fn new() -> Self {
    let fps = 60;
    let last = Instant::now();
    let render = Duration::from_micros(1_000_000/fps);
    let wakeup = Duration::from_micros(1_000_000/fps * 9/10);
    Cadence{last, render, wakeup}
  }

  fn set_frame_rate(&mut self, refresh_rate:u64) -> bool{
    let frame_time = 1_000_000_000/refresh_rate.max(1);
    let watch_interval = 1_000_000.max(frame_time/10);
    self.render = Duration::from_nanos(frame_time);
    self.wakeup = Duration::from_nanos(frame_time - watch_interval);
    refresh_rate > 0
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

#[derive(Debug)]
enum StateChange{
  Position(LogicalPosition<i32>),
  Size(LogicalSize<u32>),
  Fullscreen(bool),
  Input(char),
  Keyboard{event:String, key:String, code:u32, repeat:bool},
  Mouse(String),
  Wheel(LogicalPosition<f64>)
}

struct EventQueue{
  changes: Vec<StateChange>,
  key_modifiers: ModifiersState,
  key_repeats: HashMap<VirtualKeyCode, i32>,
  mouse_point: LogicalPosition::<i32>,
  mouse_button: Option<u16>,
}

impl EventQueue {
  fn new() -> Self {
    EventQueue{
      changes: vec![],
      key_modifiers: ModifiersState::empty(),
      key_repeats: HashMap::new(),
      mouse_point: LogicalPosition::<i32>{x:0, y:0},
      mouse_button: None,
    }
  }

  fn went_fullscreen(&mut self, did_go_fullscreen:bool){
    self.changes.push(StateChange::Fullscreen(did_go_fullscreen));
  }

  fn capture(&mut self, event:&WindowEvent, dpr:f64){
    match event{
      WindowEvent::Moved(physical_pt) => {
        let logical_pt:LogicalPosition<i32> = LogicalPosition::from_physical(*physical_pt, dpr);
        self.changes.push(StateChange::Position(logical_pt));
      }

      WindowEvent::Resized(physical_size) => {
        let logical_size:LogicalSize<u32> = LogicalSize::from_physical(*physical_size, dpr);
        self.changes.push(StateChange::Size(logical_size));
      }

      WindowEvent::ModifiersChanged(state) => {
        self.key_modifiers = *state;
      }

      WindowEvent::ReceivedCharacter(character) => {
        self.changes.push(StateChange::Input(*character));
      }

      WindowEvent::CursorEntered{..} => {
        let mouse_event = "mouseenter".to_string();
        self.changes.push(StateChange::Mouse(mouse_event));
      }

      WindowEvent::CursorLeft{..} => {
        let mouse_event = "mouseleave".to_string();
        self.changes.push(StateChange::Mouse(mouse_event));
      }

      WindowEvent::CursorMoved{position, ..} => {
        self.mouse_point = LogicalPosition::from_physical(*position, dpr);

        let mouse_event = "mousemove".to_string();
        self.changes.push(StateChange::Mouse(mouse_event));
      }

      WindowEvent::MouseWheel{delta, ..} => {
        let dxdy:LogicalPosition<f64> = match delta {
          MouseScrollDelta::PixelDelta(physical_pt) => {
            LogicalPosition::from_physical(*physical_pt, dpr)
          },
          MouseScrollDelta::LineDelta(h, v) => {
            LogicalPosition::<f64>{x:*h as f64, y:*v as f64}
          }
        };
        self.changes.push(StateChange::Wheel(dxdy));
      }

      WindowEvent::MouseInput{state, button, ..} => {
        let mouse_event = match state {
          ElementState::Pressed => "mousedown",
          ElementState::Released => "mouseup"
        }.to_string();

        self.mouse_button = match button {
          MouseButton::Left => Some(0),
          MouseButton::Middle => Some(1),
          MouseButton::Right => Some(2),
          MouseButton::Other(num) => Some(*num)
        };
        self.changes.push(StateChange::Mouse(mouse_event));
      }

      WindowEvent::KeyboardInput {
        input:
          KeyboardInput {
            scancode,
            state,
            virtual_keycode:Some(keycode),
            ..
          },
        ..
      } => {
        let (event_type, count) = match state{
          ElementState::Pressed => {
            let count = self.key_repeats.entry(*keycode).or_insert(-1);
            *count += 1;
            ("keydown", *count)
          },
          ElementState::Released => {
            self.key_repeats.remove(&keycode);
            ("keyup", 0)
          }
        };

        if event_type == "keyup" || count < 2{
          self.changes.push(StateChange::Keyboard{
            event: event_type.to_string(),
            key: from_key_code(*keycode),
            code: *scancode,
            repeat: count > 0
          });
        }

      }
      _ => {}
    }
  }

  fn digest<'a>(&mut self, cx: &mut FunctionContext<'a>) -> Vec<Handle<'a, JsValue>>{

    let mut payload:Vec<Handle<JsValue>> = (0..17).map(|i|
      //   0–5: x, y, w, h, fullscreen, [alt, ctrl, meta, shift]
      //  6–10: input, keyEvent, key, code, repeat,
      // 11–14: [mouseEvents], mouseX, mouseY, button,
      // 15–16: wheelX, wheelY
      cx.undefined().upcast::<JsValue>()
    ).collect();

    let mut need_mods = false;
    let mut mouse_events = vec![];

    for change in &self.changes {
      match change{
        StateChange::Position(LogicalPosition{x, y}) => {
          payload[0] = cx.number(*x).upcast::<JsValue>(); // x
          payload[1] = cx.number(*y).upcast::<JsValue>(); // y
        }
        StateChange::Size(LogicalSize{width, height}) => {
          payload[2] = cx.number(*width).upcast::<JsValue>();  // width
          payload[3] = cx.number(*height).upcast::<JsValue>(); // height
        }
        StateChange::Fullscreen(flag) => {
          payload[4] = cx.boolean(*flag).upcast::<JsValue>(); // fullscreen
        }
        StateChange::Input(character) => {
          need_mods = true;
          payload[6] = cx.string(character.to_string()).upcast::<JsValue>(); // input
        }
        StateChange::Keyboard{event, key, code, repeat} => {
          need_mods = true;
          payload[7] = cx.string(event).upcast::<JsValue>();     // keyup | keydown
          payload[8] = cx.string(key).upcast::<JsValue>();       // key
          payload[9] = cx.number(*code).upcast::<JsValue>();     // code
          payload[10] = cx.boolean(*repeat).upcast::<JsValue>(); // repeat
        }
        StateChange::Mouse(event_type) => {
          need_mods = true;
          let event_name = cx.string(event_type).upcast::<JsValue>();
          mouse_events.push(event_name);
        }
        StateChange::Wheel(delta) => {
          payload[15] = cx.number(delta.x).upcast::<JsValue>(); // wheelX
          payload[16] = cx.number(delta.y).upcast::<JsValue>(); // wheelY
        }
      }
    }

    if !mouse_events.is_empty(){
      let event_list = JsArray::new(cx, mouse_events.len() as u32);
      for (i, obj) in mouse_events.iter().enumerate() {
        event_list.set(cx, i as u32, *obj).unwrap();
      }
      payload[11] = event_list.upcast::<JsValue>();

      let LogicalPosition{x, y} = self.mouse_point;
      payload[12] = cx.number(x).upcast::<JsValue>(); // mouseX
      payload[13] = cx.number(y).upcast::<JsValue>(); // mouseY

      if let Some(button_id) = self.mouse_button{
        payload[14] = cx.number(button_id).upcast::<JsValue>();   // button
        self.mouse_button = None;
      }
    }

    if need_mods{
      let mod_info = JsArray::new(cx, 4);
      let mod_info_vec = vec![
        cx.boolean(self.key_modifiers.alt()).upcast::<JsValue>(),   // altKey
        cx.boolean(self.key_modifiers.ctrl()).upcast::<JsValue>(),  // ctrlKey
        cx.boolean(self.key_modifiers.logo()).upcast::<JsValue>(),  // metaKey
        cx.boolean(self.key_modifiers.shift()).upcast::<JsValue>(), // shiftKey
      ];
      for (i, obj) in mod_info_vec.iter().enumerate() {
          mod_info.set(cx, i as u32, *obj).unwrap();
      }
      payload[5] = mod_info.upcast::<JsValue>();
    }

    self.changes.clear();
    payload
  }

}

pub fn begin_display_loop(mut cx: FunctionContext) -> JsResult<JsUndefined> {
  let context = cx.argument::<BoxedContext2D>(0)?;
  let dispatch = cx.argument::<JsFunction>(1)?;
  let animate = cx.argument::<JsFunction>(2)?;
  let matte = color_arg(&mut cx, 3).unwrap_or(Color::BLACK);

  let mut runloop = EventLoop::new();
  let mut event_queue = EventQueue::new();
  let mut view = View::new(&runloop, context, matte);
  let null = cx.null();

  // runloop state
  let mut cadence = Cadence::new();
  let mut change_queue:Vec<StateChange> = vec![];
  let mut new_loop = true;
  let mut is_stale = true;
  let mut is_fullscreen = false;
  let mut is_animated = false;
  let mut is_done = false;

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
          view.context.window().set_visible(true);
        },
        Err(_) => is_done = true
      }

      new_loop = false;
    }

    match event {
      Event::NewEvents(start_cause) => {
        if is_done{
          *control_flow = ControlFlow::Exit;
        }else if is_animated{
          *control_flow = cadence.on_next_frame(||
            view.context.window().request_redraw()
          );
        }
      }

      Event::WindowEvent{event, window_id} => match event {
        WindowEvent::Resized(physical_size) => {
          event_queue.capture(&event, view.dpr());

          if is_fullscreen != view.in_fullscreen() {
            event_queue.went_fullscreen(!is_fullscreen);
            is_fullscreen = !is_fullscreen;
          }
          view.resize(physical_size);
        }
        WindowEvent::CloseRequested => {
          is_done = true;
        }
        WindowEvent::KeyboardInput {
          input: KeyboardInput {
            scancode, state, virtual_keycode: Some(keycode), ..
          }, ..
        } => {
          if keycode==VirtualKeyCode::Escape {
            if view.in_fullscreen(){
              view.go_fullscreen(false);
              event_queue.went_fullscreen(false);
              is_fullscreen = false;
            }else{
              is_done = true;
            }
          }else{
            event_queue.capture(&event, view.dpr());
          }
        }
        _ => {
          // all other WindowEvents
          event_queue.capture(&event, view.dpr());
        }
      }

      Event::MainEventsCleared => {
        if !event_queue.changes.is_empty(){

          // dispatch UI event-related state changes
          let changes = event_queue.digest(&mut cx);
          match dispatch.call(&mut cx, null, changes){
            Ok(result) => {
              let (should_quit, to_fullscreen, to_fps) = view.handle_events(&mut cx, result);
              if to_fullscreen != is_fullscreen{
                event_queue.went_fullscreen(to_fullscreen);
                event_queue.key_repeats.clear() // keyups don't get delivered during the transition apparently?
              }

              is_animated = cadence.set_frame_rate(to_fps);
              is_fullscreen = to_fullscreen;
              is_done = should_quit;

              if !is_animated{
                view.context.window().request_redraw();
              }
            },
            Err(_) => is_done = true
          }
        }

      }
      Event::RedrawRequested(window_id) => {
        view.redraw();
        is_stale = true;
      },
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

      _ => {
        // all other generic Events
      }
    }
  });

  Ok(cx.undefined())
}
