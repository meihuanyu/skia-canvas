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
use skia_safe::{Color, ColorType, Surface, Picture};

use glutin::platform::run_return::EventLoopExtRunReturn;
use glutin::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent, ModifiersState, ElementState, MouseButton, MouseScrollDelta};
use glutin::dpi::{LogicalSize, PhysicalSize, LogicalPosition, PhysicalPosition};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::{WindowBuilder, Fullscreen};
use glutin::GlProfile;
use gl::types::*;
use gl_rs as gl;

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
}

impl View{
  fn new(runloop:&EventLoop<()>, c2d:Handle<BoxedContext2D>, title:&str) -> Self{
    let wb = WindowBuilder::new().with_title(title);
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

    let (gl, surface) = View::gl_surface(&context);
    View{
      context,
      ident: ctx.ident(),
      title: title.to_string(),
      pict: ctx.get_picture(None).unwrap(),
      dims: (width, height),
      surface: RefCell::new(surface),
      gl: RefCell::new(gl)
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

  fn redraw(&self){
    let mut surface = self.surface.borrow_mut();
    let canvas = surface.canvas();

    let physical_size = self.context.window().inner_size();
    let sf = physical_size.height as f32 / self.dims.1;
    let indent = (physical_size.width as f32 - self.dims.0 * sf) / 2.0;

    canvas.clear(Color::TRANSPARENT);
    canvas.save();
    canvas.translate((indent, 0.0)).scale((sf, sf));
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
    let mut should_quit = false;
    let mut to_fullscreen = false;
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

        if let Ok(title) = vals[1].downcast::<JsString, _>(cx){
          let title = title.value(cx);
          if self.title != title{
            self.title = title;
            self.context.window().set_title(&self.title);
          }
        }

        if let Ok(active) = vals[2].downcast::<JsBoolean, _>(cx){
          if !active.value(cx){ should_quit = true }
        }

        if let Ok(is_full) = vals[3].downcast::<JsBoolean, _>(cx){
          let is_full = is_full.value(cx);
          let was_full = self.context.window().fullscreen().is_some();
          if is_full != was_full{
            match is_full{
              true => self.context.window().set_fullscreen( Some(Fullscreen::Borderless(None)) ),
              false => self.context.window().set_fullscreen( None )
            }
          }
          to_fullscreen = is_full
        }

        if let Ok(fps) = vals[4].downcast::<JsNumber, _>(cx){
          to_fps = fps.value(cx) as u64;
        }

        let dpr = self.dpr() as f32;
        let old_pos = self.context.window().outer_position().unwrap();
        let new_pos:Vec<i32> = floats_in(cx, &vals[5..7]).iter().map(|d| (*d * dpr) as i32).collect();
        if let [x, y] = new_pos.as_slice(){
          if *x != old_pos.x || *y != old_pos.y{
            let position = PhysicalPosition::<i32>::new(*x, *y);
            self.context.window().set_outer_position(position)
          }
        }

        let old_dims = self.context.window().inner_size();
        let new_dims:Vec<u32> = floats_in(cx, &vals[7..9]).iter().map(|d| (*d * dpr) as u32).collect();
        if let [width, height] = new_dims.as_slice(){
          if *width != old_dims.width || *height != old_dims.height{
            let size = PhysicalSize::<u32>::new(*width, *height);
            self.context.window().set_inner_size(size)
          }
        }

      }
    }

    (should_quit, to_fullscreen, to_fps)
  }
}

struct Cadence{
  fps: u64,
  last: Instant,
  shutter: Duration,
}

impl Cadence{
  fn new() -> Self {
    let fps = 60;
    let last = Instant::now();
    let shutter = Duration::from_micros(1_000_000/fps);
    Cadence{fps, last, shutter}
  }

  fn next(&mut self){
    self.last = Instant::now();
  }

  fn set_frame_rate(&mut self, refresh_rate:u64) -> bool{
    self.shutter = Duration::from_micros(1_000_000/refresh_rate.max(1));
    self.fps = refresh_rate;
    refresh_rate > 0
  }

  fn render(&self) -> bool{   self.last.elapsed() >= self.shutter }
  fn wakeup(&self) -> bool{   self.last.elapsed() >= self.shutter * 9/10 }
  fn sleep(&self) -> Instant{ self.last            + self.shutter * 9/10 }
}

enum StateChange{
  Position(LogicalPosition<i32>),
  Size(LogicalSize<u32>),
  Fullscreen(bool),
  Input(char),
  Keyboard{event:String, key:String, code:u32, repeat:bool},
  Mouse(String),
  Wheel(LogicalPosition<f64>)
}

pub fn begin_display_loop(mut cx: FunctionContext) -> JsResult<JsUndefined> {
  let context = cx.argument::<BoxedContext2D>(0)?;
  let title = cx.argument::<JsString>(1)?.value(&mut cx);
  let callback = cx.argument::<JsFunction>(2)?;
  let animate = cx.argument::<JsFunction>(3)?;
  let init_fps = cx.argument::<JsNumber>(4)?.value(&mut cx) as u64;

  let mut runloop = EventLoop::new();
  let mut view = View::new(&runloop, context, &title);
  let null = cx.null();

  // animation
  let mut cadence = Cadence::new();
  let mut is_animated = cadence.set_frame_rate(init_fps);

  // key events
  let mut modifiers = ModifiersState::empty();
  let mut repeats:HashMap<VirtualKeyCode, i32> = HashMap::new();

  // mouse events
  let mut mouse_point = LogicalPosition::<i32>{x:0, y:0};
  let mut mouse_button:Option<u16> = None;

  // runloop state
  let mut change_queue = vec![];
  let mut needs_render = true;
  let mut is_fullscreen = false;
  let mut is_done = false;

  runloop.run_return(|event, _, control_flow| {
    // println!("{:?}", event);

    match event {
      Event::NewEvents(start_cause) => {
        if is_done{
          *control_flow = ControlFlow::Exit;
        }else if is_animated{
          if cadence.render(){
            cadence.next();
            view.context.window().request_redraw();
          }else if cadence.wakeup(){
            *control_flow = ControlFlow::Poll;
          }else{
            *control_flow = ControlFlow::WaitUntil(cadence.sleep());
          }
        }
      }
      Event::WindowEvent { event, window_id } => match event {
        WindowEvent::Moved(physical_pt) => {
          let logical_pt:LogicalPosition<i32> = LogicalPosition::from_physical(physical_pt, view.dpr());
          change_queue.push(StateChange::Position(logical_pt));
        }

        WindowEvent::Resized(physical_size) => {
          let logical_size:LogicalSize<u32> = LogicalSize::from_physical(physical_size, view.dpr());
          change_queue.push(StateChange::Size(logical_size));

          if is_fullscreen != view.context.window().fullscreen().is_some() {
            change_queue.push(StateChange::Fullscreen(!is_fullscreen));
          }
          view.resize(physical_size);
        }

        WindowEvent::ModifiersChanged(state) => {
          modifiers = state;
        }

        WindowEvent::CloseRequested => {
          is_done = true;
        }

        WindowEvent::ReceivedCharacter(character) => {
          change_queue.push(StateChange::Input(character));
        }

        WindowEvent::CursorEntered{..} => {
          let mouse_event = "mouseleave".to_string();
          change_queue.push(StateChange::Mouse(mouse_event));
        }

        WindowEvent::CursorLeft{..} => {
          let mouse_event = "mouseleave".to_string();
          change_queue.push(StateChange::Mouse(mouse_event));
        }

        WindowEvent::CursorMoved{position, ..} => {
          mouse_point = LogicalPosition::from_physical(position, view.dpr());

          let mouse_event = "mousemove".to_string();
          change_queue.push(StateChange::Mouse(mouse_event));
        }

        WindowEvent::MouseWheel{delta, ..} => {
          let dxdy:LogicalPosition<f64> = match delta {
            MouseScrollDelta::PixelDelta(physical_pt) => {
              LogicalPosition::from_physical(physical_pt, view.dpr())
            },
            MouseScrollDelta::LineDelta(h, v) => {
              LogicalPosition::<f64>{x:h as f64, y:v as f64}
            }
          };
          change_queue.push(StateChange::Wheel(dxdy));
        }

        WindowEvent::MouseInput{state, button, ..} => {
          let mouse_event = match state {
            ElementState::Pressed => "mousedown",
            ElementState::Released => "mouseup"
          }.to_string();

          mouse_button = match button {
            MouseButton::Left => Some(0),
            MouseButton::Middle => Some(1),
            MouseButton::Right => Some(2),
            MouseButton::Other(num) => Some(num)
          };
          change_queue.push(StateChange::Mouse(mouse_event));
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
          if keycode==VirtualKeyCode::Escape {
            if view.context.window().fullscreen().is_some(){
              view.context.window().set_fullscreen(None);
              change_queue.push(StateChange::Fullscreen(false));
            }else{
              is_done = true;
            }
          }else if modifiers.logo() && keycode==VirtualKeyCode::Q{
            is_done = true;
        }else if modifiers.logo() && keycode==VirtualKeyCode::F{
            if !is_fullscreen{
              view.context.window().set_fullscreen( Some(Fullscreen::Borderless(None)) );
              change_queue.push(StateChange::Fullscreen(true));
            }
          }else{
            let (event_type, count) = match state{
              ElementState::Pressed => {
                let count = repeats.entry(keycode).or_insert(-1);
                *count += 1;
                ("keydown", *count)
              },
              ElementState::Released => {
                repeats.remove(&keycode);
                ("keyup", 0)
              }
            };

            if event_type == "keyup" || count < 2{
              change_queue.push(StateChange::Keyboard{
                event: event_type.to_string(),
                key: from_key_code(keycode),
                code: scancode,
                repeat: count > 0
              });
            }
          }

        }
        _ => (),
      },
      Event::MainEventsCleared => {
        // relay the queued events to js
        if !change_queue.is_empty(){
          //   0–5: x, y, w, h, fullscreen, [alt, ctrl, meta, shift]
          //  6–10: input, keyEvent, key, code, repeat,
          // 11–14: [mouseEvents], mouseX, mouseY, button,
          // 15–16: wheelX, wheelY
          let mut payload:Vec<Handle<JsValue>> = (0..17).map(|i|
            cx.undefined().upcast::<JsValue>()
          ).collect();

          let mut need_mods = false;
          let mut mouse_events = vec![];

          for change in &change_queue {
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
                is_fullscreen = *flag;
              }
              StateChange::Input(character) => {
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
            let event_list = JsArray::new(&mut cx, mouse_events.len() as u32);
            for (i, obj) in mouse_events.iter().enumerate() {
              event_list.set(&mut cx, i as u32, *obj).unwrap();
            }
            payload[11] = event_list.upcast::<JsValue>();
            payload[12] = cx.number(mouse_point.x).upcast::<JsValue>(); // mouseX
            payload[13] = cx.number(mouse_point.y).upcast::<JsValue>(); // mouseY
            if let Some(button_id) = mouse_button{
              payload[14] = cx.number(button_id).upcast::<JsValue>();   // button
              mouse_button = None;
            }
          }

          if need_mods{
            let mod_info = JsArray::new(&mut cx, 4);
            let mod_info_vec = vec![
              cx.boolean(modifiers.alt()).upcast::<JsValue>(),   // altKey
              cx.boolean(modifiers.ctrl()).upcast::<JsValue>(),  // ctrlKey
              cx.boolean(modifiers.logo()).upcast::<JsValue>(),  // metaKey
              cx.boolean(modifiers.shift()).upcast::<JsValue>(), // shiftKey
            ];
            for (i, obj) in mod_info_vec.iter().enumerate() {
                mod_info.set(&mut cx, i as u32, *obj).unwrap();
            }
            payload[5] = mod_info.upcast::<JsValue>();
          }

          // relay UI event-related state changes
          if let Ok(result) = callback.call(&mut cx, null, payload){
            let (should_quit, to_fullscreen, to_fps) = view.handle_events(&mut cx, result);
            if to_fullscreen != is_fullscreen{
              repeats.clear(); // keyups don't get delivered during the transition apparently?
            }

            is_animated = cadence.set_frame_rate(to_fps);
            is_fullscreen = to_fullscreen;
            is_done = should_quit;

            if !is_animated{
              view.context.window().request_redraw();
            }
          }

          change_queue.clear();
        }

      }
      Event::RedrawRequested(window_id) => {
        view.redraw();
        needs_render = true;
      },
      Event::RedrawEventsCleared => {
        if needs_render && is_animated{
          // call the `frame` event handler
          match animate.call(&mut cx, null, argv()){
            Ok(result) => {
              let (should_quit, to_fps) = view.animate(&mut cx, result);
              is_animated = cadence.set_frame_rate(to_fps);
              is_done = should_quit;
              needs_render = false;
            },
            Err(e) => {
              println!("Error {}", e);
              is_done = true;
            }
          }
        }

      },
      _ => {}
    }

  });

  Ok(cx.undefined())
}
