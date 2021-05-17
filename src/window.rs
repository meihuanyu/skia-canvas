#![allow(unused_mut)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use neon::prelude::*;

use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::{BackendRenderTarget, SurfaceOrigin, DirectContext};
use skia_safe::{Image as SkImage, ImageInfo, Color, ColorType,
        AlphaType, Data, Surface, Rect, Picture, Paint, PaintStyle};

use glutin::{PossiblyCurrent};
use glutin::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent, ModifiersState, ElementState};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::{WindowBuilder, WindowId, Fullscreen};
use glutin::dpi::{Size, LogicalSize, PhysicalSize, LogicalPosition, PhysicalPosition};
use glutin::GlProfile;
use glutin::platform::run_return::EventLoopExtRunReturn;
type WindowedContext = glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>;

use gl::types::*;
use gl_rs as gl;

use crate::context::{Context2D, BoxedContext2D};
use crate::utils::*;

fn create_surface(windowed_context: &WindowedContext, gl_context:&mut DirectContext) -> skia_safe::Surface {
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
  Surface::from_backend_render_target(
    gl_context,
    &backend_render_target,
    SurfaceOrigin::BottomLeft,
    ColorType::RGBA8888,
    None,
    None,
  )
  .unwrap()
}

struct View{
  pict:Picture,
  dims:(f32, f32),
  title:String,
  context:WindowedContext,
  surface:RefCell<Surface>,
  gl:RefCell<DirectContext>,
}

impl View{
  fn new(runloop:&EventLoop<()>, c2d:Handle<BoxedContext2D>, title:&str) -> Self{
    let mut ctx = c2d.borrow_mut();
    let pict = ctx.get_picture(None).unwrap();
    let bounds = ctx.bounds;
    let (width, height) = (bounds.width(), bounds.height());

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
    context.window()
      .set_inner_size(Size::new(LogicalSize::new(width, height)));

    let title = title.to_string();
    let gl = RefCell::new(DirectContext::new_gl(None, None).unwrap());
    let surface = RefCell::new(create_surface(&context, &mut gl.borrow_mut()));
    View{dims:(width, height), pict, title, context, surface, gl}
  }

  fn dpr(&self) -> f64{
    self.context.window().scale_factor() as f64
  }

  fn resize(&self, physical_size:PhysicalSize<u32>){
    let mut gr_context = DirectContext::new_gl(None, None).unwrap();
    let mut surface = create_surface(&self.context, &mut gr_context);
    self.context.resize(physical_size);
    self.surface.replace(surface);
    self.gl.replace(gr_context);
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

  fn update(&mut self, cx:&mut FunctionContext, result:Handle<JsValue>) -> Option<bool>{
    if let Ok(array) = result.downcast::<JsArray, _>(cx){
      if let Ok(vals) = array.to_vec(cx){

        if let Ok(c2d) = vals[0].downcast::<BoxedContext2D, _>(cx){
          let mut ctx = c2d.borrow_mut();
          let pict = ctx.get_picture(None).unwrap();
          let bounds = ctx.bounds;
          self.pict = pict;
          self.dims = (bounds.width(), bounds.height());
        }

        if let Ok(title) = vals[1].downcast::<JsString, _>(cx){
          self.title = title.value(cx);
          self.context.window().set_title(&self.title);
        }

        if let Ok(active) = vals[2].downcast::<JsBoolean, _>(cx){
          if !active.value(cx){ return Some(true) }
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
        }

        let dpr = self.dpr() as f32;
        let old_pos = self.context.window().outer_position().unwrap();
        let new_pos:Vec<i32> = floats_in(cx, &vals[4..6]).iter().map(|d| (*d * dpr) as i32).collect();
        if let [x, y] = new_pos.as_slice(){
          if *x != old_pos.x || *y != old_pos.y{
            let position = PhysicalPosition::<i32>::new(*x, *y);
            self.context.window().set_outer_position(position)
          }
        }

        let old_dims = self.context.window().inner_size();
        let new_dims:Vec<u32> = floats_in(cx, &vals[6..8]).iter().map(|d| (*d * dpr) as u32).collect();
        if let [width, height] = new_dims.as_slice(){
          if *width != old_dims.width || *height != old_dims.height{
            let size = PhysicalSize::<u32>::new(*width, *height);
            self.context.window().set_inner_size(size)
          }

        }

      }
    }
    None
  }
}

pub fn begin_display_loop(mut cx: FunctionContext) -> JsResult<JsUndefined> {
  let context = cx.argument::<BoxedContext2D>(0)?;
  let title = cx.argument::<JsString>(1)?.value(&mut cx);
  let callback = cx.argument::<JsFunction>(2)?;
  let animate = cx.argument::<JsFunction>(3)?;
  let that = cx.null();

  let mut runloop = EventLoop::new();
  let mut view = View::new(&runloop, context, &title);

  let mut frame = 0;
  let mut modifiers = ModifiersState::empty();
  let mut repeats:HashMap<VirtualKeyCode, i32> = HashMap::new();

  runloop.run_return(|event, _, control_flow| {
    // println!("{:?}", event);
    *control_flow = ControlFlow::Wait;

    let mut js_event = None;

    match event {
      Event::LoopDestroyed => (),
      Event::WindowEvent { event, window_id } => match event {
        WindowEvent::Moved(physical_pt) => {
          let logical_pt:LogicalPosition<u32> = LogicalPosition::from_physical(physical_pt, view.dpr());
          js_event = Some(vec![
            cx.string("move").upcast::<JsValue>(),
            cx.number(logical_pt.x).upcast::<JsValue>(),
            cx.number(logical_pt.y).upcast::<JsValue>(),
          ]);
        }

        WindowEvent::Resized(physical_size) => {
          let logical_size:LogicalSize<u32> = LogicalSize::from_physical(physical_size, view.dpr());
          js_event = Some(vec![
            cx.string("resize").upcast::<JsValue>(),
            cx.number(logical_size.width).upcast::<JsValue>(),
            cx.number(logical_size.height).upcast::<JsValue>(),
          ]);

          view.resize(physical_size);
        }

        WindowEvent::ModifiersChanged(state) => {
          modifiers = state;
        }

        WindowEvent::CloseRequested => {
          *control_flow = ControlFlow::Exit
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
              js_event = Some(vec![
                cx.string("fullscreen").upcast::<JsValue>(),
                cx.boolean(false).upcast::<JsValue>()
              ])
            }else{
              *control_flow = ControlFlow::Exit
            }
          }else if modifiers.logo() && keycode==VirtualKeyCode::Q{
            *control_flow = ControlFlow::Exit
          }else if modifiers.logo() && keycode==VirtualKeyCode::F{
            view.context.window().set_fullscreen( Some(Fullscreen::Borderless(None)) );
            js_event = Some(vec![
              cx.string("fullscreen").upcast::<JsValue>(),
              cx.boolean(true).upcast::<JsValue>()
            ])
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
              js_event = Some(vec![
                cx.string(event_type).upcast::<JsValue>(),             // keyup | keydown
                cx.string(from_key_code(keycode)).upcast::<JsValue>(), // key
                cx.number(scancode).upcast::<JsValue>(),               // code
                cx.boolean(count > 0).upcast::<JsValue>(),             // repeat
                cx.boolean(modifiers.alt()).upcast::<JsValue>(),       // altKey
                cx.boolean(modifiers.ctrl()).upcast::<JsValue>(),      // ctrlKey
                cx.boolean(modifiers.logo()).upcast::<JsValue>(),      // metaKey
                cx.boolean(modifiers.shift()).upcast::<JsValue>(),     // shiftKey
              ]);
            }
          }

        }
        _ => (),
      },
      Event::RedrawRequested(window_id) => {
          view.redraw();
      }
      _ => (),
    }

    if let Some(args) = js_event{
      if let Ok(result) = callback.call(&mut cx, that, args){
        let should_quit = view.update(&mut cx, result);
        if should_quit.is_some(){
          *control_flow = ControlFlow::Exit
        }
      }
    }
  });

  Ok(cx.undefined())
}
