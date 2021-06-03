use std::cell::RefCell;
use std::convert::TryInto;
use neon::prelude::*;
use crossbeam::channel::Receiver;

use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::{BackendRenderTarget, SurfaceOrigin, DirectContext};
use skia_safe::{Rect, Color, ColorType, Surface, Picture};

use glutin::dpi::{LogicalSize, PhysicalSize, LogicalPosition};
use glutin::event_loop::{EventLoop, EventLoopProxy};
use glutin::window::{WindowBuilder, Fullscreen};
use glutin::event::{Event, WindowEvent};
use glutin::GlProfile;
use gl::types::*;

use crate::context::{BoxedContext2D};
use crate::utils::to_cursor_icon;
use super::CanvasEvent;


type WindowedContext = glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>;

pub struct View{
  context: WindowedContext,
  ident: (usize, usize),
  pict: Picture,
  dims: (f32, f32),
  title: String,
  surface: RefCell<Surface>,
  gl: RefCell<DirectContext>,
  needs_redraw: bool,
  backdrop: Color,
  js_events:Receiver<CanvasEvent>,
  ui_events: EventLoopProxy<CanvasEvent>,
}

impl View{
  pub fn new(
    runloop:&EventLoop<CanvasEvent>,
    c2d:Handle<BoxedContext2D>,
    js_events:Receiver<CanvasEvent>,
    backdrop:Option<Color>,
    width:f32,
    height:f32
  ) -> Self{
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

    let mut ctx = c2d.borrow_mut();
    let (gl, surface) = View::gl_surface(&context);
    View{
      context,
      ident: ctx.ident(),
      title: "".to_string(),
      pict: ctx.get_picture(None).unwrap(),
      dims: (ctx.bounds.width(), ctx.bounds.height()),
      surface: RefCell::new(surface),
      gl: RefCell::new(gl),
      needs_redraw: true,
      backdrop,
      ui_events: runloop.create_proxy(),
      js_events,
    }
  }

  fn gl_surface(windowed_context: &WindowedContext) -> (DirectContext, Surface) {
    let mut gl_context = DirectContext::new_gl(None, None).expect("Could not initialize OpenGL");

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

  pub fn handle_event(&mut self, event:&CanvasEvent){
    let mut window = self.context.window();
    let dpr = window.scale_factor() as f64;

    // For now, listen to the channel for Resized WindowEvents repackaged by the Window
    // and Fullscreen changes made in a js event handler. Eventually all events will come
    // from the channel rather than handle_event being invoked on the main thread...
    for e in self.js_events.try_iter(){
      match e {
        CanvasEvent::Resized(physical_size) => {
          self.resize(physical_size);
          self.redraw();
          let is_fullscreen = window.fullscreen().is_some();
          self.ui_events.send_event(CanvasEvent::Fullscreen(is_fullscreen)).ok();
        },
        CanvasEvent::Fullscreen(to_fullscreen) => {
          match to_fullscreen{
            true => window.set_fullscreen( Some(Fullscreen::Borderless(None)) ),
            false => window.set_fullscreen( None )
          }
        },
        _ => {}
      }
    }

    match event{
      CanvasEvent::Visible(visible) => window.set_visible(*visible),
      CanvasEvent::Title(title) => window.set_title(title),
      CanvasEvent::Size(size) => window.set_inner_size(*size),
      CanvasEvent::Position(position) => window.set_outer_position(*position),

      CanvasEvent::Page(page) => {
        if page.ident != self.ident{
          if let Some(pict) = page.get_picture(){
            self.pict = pict;
            self.dims = (page.bounds.width(), page.bounds.height());
            self.ident = page.ident;
            self.needs_redraw = true;
          }
        }
      }

      CanvasEvent::Cursor(cursor_icon) => {
        window.set_cursor_visible(cursor_icon.is_some());
        if let Some(icon) = cursor_icon{
          window.set_cursor_icon(*icon);
        }
      },

      CanvasEvent::Render => {
        // if self.needs_redraw{
          self.redraw();
          self.needs_redraw = false;
          self.context.window().request_redraw();
        // }
      }

      // Heartbeat,
      // FrameRate(u64),
      // Close,

      _ => {}
    }


  }
}