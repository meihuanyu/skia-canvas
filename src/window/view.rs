use std::cell::RefCell;
use std::convert::TryInto;
use neon::prelude::*;

use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::{BackendRenderTarget, SurfaceOrigin, DirectContext};
use skia_safe::{Rect, Color, ColorType, Surface, Picture};

use glutin::dpi::{LogicalSize, PhysicalSize, LogicalPosition};
use glutin::event_loop::{EventLoop};
use glutin::window::{WindowBuilder, Fullscreen};
use glutin::GlProfile;
use gl::types::*;

use crate::context::{BoxedContext2D};
use crate::utils::to_cursor_icon;

type WindowedContext = glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>;

pub struct View{
  context: WindowedContext,
  ident: (usize, usize),
  pict: Picture,
  dims: (f32, f32),
  title: String,
  surface: RefCell<Surface>,
  gl: RefCell<DirectContext>,
  backdrop: Color
}

impl View{
  pub fn new(runloop:&EventLoop<()>, c2d:Handle<BoxedContext2D>, backdrop:Option<Color>) -> Self{
    let backdrop = backdrop.unwrap_or(Color::BLACK);

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
      let screen_size = LogicalSize::<f32>::from_physical(
        monitor.size(), monitor.scale_factor()
      );
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

  pub fn dpr(&self) -> f64{
    self.context.window().scale_factor() as f64
  }

  pub fn resize(&self, physical_size:PhysicalSize<u32>){
    let (gl, surface) = View::gl_surface(&self.context);
    self.context.resize(physical_size);
    self.surface.replace(surface);
    self.gl.replace(gl);
  }

  pub fn go_visible(&mut self, to_visible:bool){
    self.context.window().set_visible(to_visible);
  }

  pub fn in_fullscreen(&self) -> bool {
    self.context.window().fullscreen().is_some()
  }

  pub fn request_redraw(&self){
    self.context.window().request_redraw()
  }

  pub fn redraw(&self){
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

  pub fn animate(&mut self, cx:&mut FunctionContext, result:Handle<JsValue>) -> (bool, u64){
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

  pub fn handle_events(&mut self, cx:&mut FunctionContext, result:Handle<JsValue>) -> (bool, bool, u64){
    let window = self.context.window();
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
