#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_imports)]
use std::cmp;
use std::f32::consts::PI;
use core::ops::Range;
use neon::prelude::*;
use neon::result::Throw;
use neon::object::This;
use css_color::Rgba;
use skia_safe::{
  Path, Matrix, Point, Color, Color4f, RGB, Rect, FontArguments,
  font_style::{FontStyle, Weight, Width, Slant},
  font_arguments::{VariationPosition, variation_position::{Coordinate}}
};

//
// meta-helpers
//

fn arg_num(o:usize) -> String{
  // let n = (o + 1) as i32; // we're working with zero-bounded idxs
  let n = o; // arg 0 is always self, so no need to increment the idx
  let ords = ["st","nd","rd"];
  let slot = ((n+90)%100-10)%10 - 1;
  let suffix = if (0..=2).contains(&slot) { ords[slot as usize] } else { "th" };
  format!("{}{}", n, suffix)
}

pub fn argv<'a>() -> Vec<Handle<'a, JsValue>>{
  let list:Vec<Handle<JsValue>> = Vec::new();
  list
}

// pub fn clamp(val: f32, min:f64, max:f64) -> f32{
//   let min = min as f32;
//   let max = max as f32;
//   if val < min { min } else if val > max { max } else { val }
// }

pub fn almost_equal(a: f32, b: f32) -> bool{
  (a-b).abs() < 0.00001
}

pub fn to_degrees(radians: f32) -> f32{
  radians / PI * 180.0
}

pub fn to_radians(degrees: f32) -> f32{
  degrees / 180.0 * PI
}

// pub fn symbol<'a>(cx: &mut FunctionContext<'a>, symbol_name: &str) -> JsResult<'a, JsValue> {
//   let global = cx.global();
//   let symbol_ctor = global
//       .get(cx, "Symbol")?
//       .downcast::<JsObject, _>(cx)
//       .or_throw(cx)?
//       .get(cx, "for")?
//       .downcast::<JsFunction, _>(cx)
//       .or_throw(cx)?;

//   let symbol_label = cx.string(symbol_name);
//   let sym = symbol_ctor.call(cx, global, vec![symbol_label])?;
//   Ok(sym)
// }

//
// strings
//

pub fn strings_in(cx: &mut FunctionContext, vals: &[Handle<JsValue>]) -> Vec<String>{
  let mut strs:Vec<String> = Vec::new();
  for (i, val) in vals.iter().enumerate() {
    if let Ok(txt) = val.downcast::<JsString, _>(cx){
      let val = txt.value(cx);
      strs.push(val);
    }
  }
  strs
}

pub fn strings_at_key(cx: &mut FunctionContext, obj: &Handle<JsObject>, attr:&str) -> Result<Vec<String>, Throw>{
  let array = obj.get(cx, attr)?.downcast::<JsArray, _>(cx).or_throw(cx)?.to_vec(cx)?;
  Ok(strings_in(cx, &array))
}

pub fn string_for_key(cx: &mut FunctionContext, obj: &Handle<JsObject>, attr:&str) -> Result<String, Throw>{
  let key = cx.string(attr);
  match obj.get(cx, key)?.downcast::<JsString, _>(cx){
    Ok(s) => Ok(s.value(cx)),
    Err(_e) => cx.throw_type_error(format!("Exptected a string for \"{}\"", attr))
  }
}

pub fn opt_string_arg(cx: &mut FunctionContext, idx: usize) -> Option<String>{
  match cx.argument_opt(idx as i32) {
    Some(arg) => match arg.downcast::<JsString, _>(cx) {
      Ok(v) => Some(v.value(cx)),
      Err(_e) => None
    },
    None => None
  }
}

pub fn string_arg_or(cx: &mut FunctionContext, idx: usize, default:&str) -> String{
  match opt_string_arg(cx, idx){
    Some(v) => v,
    None => String::from(default)
  }
}

pub fn string_arg<'a>(cx: &mut FunctionContext<'a>, idx: usize, attr:&str) -> Result<String, Throw> {
  let exists = cx.len() > idx as i32;
  match opt_string_arg(cx, idx){
    Some(v) => Ok(v),
    None => cx.throw_type_error(
      if exists { format!("{} must be a string", attr) }
      else { format!("Missing argument: expected a string for {} ({} arg)", attr, arg_num(idx)) }
    )
  }
}

pub fn strings_to_array<'a>(cx: &mut FunctionContext<'a>, strings: &[String]) -> JsResult<'a, JsArray> {
  let array = JsArray::new(cx, strings.len() as u32);
  for (i, val) in strings.iter().enumerate() {
    let num = cx.string(val.as_str());
    array.set(cx, i as u32, num)?;
  }
  Ok(array)
}

// Convert from byte-indices to char-indices for a given UTF-8 string
pub fn string_idx_range(text: &str, begin: usize, end: usize) -> Range<usize>{
  let start = text[0..begin].chars().count();
  let end = start + text[begin..end].chars().count();
  Range{ start, end }
}

//
// bools
//

pub fn opt_bool_arg(cx: &mut FunctionContext, idx: usize) -> Option<bool>{
  match cx.argument_opt(idx as i32) {
    Some(arg) => match arg.downcast::<JsBoolean, _>(cx) {
      Ok(v) => Some(v.value(cx)),
      Err(_e) => None
    },
    None => None
  }
}

pub fn bool_arg_or(cx: &mut FunctionContext, idx: usize, default:bool) -> bool{
  match opt_bool_arg(cx, idx){
    Some(v) => v,
    None => default
  }
}

pub fn bool_arg(cx: &mut FunctionContext, idx: usize, attr:&str) -> Result<bool, Throw>{
  let exists = cx.len() > idx as i32;
  match opt_bool_arg(cx, idx){
    Some(v) => Ok(v),
    None => cx.throw_type_error(
      if exists { format!("{} must be a boolean", attr) }
      else { format!("Missing argument: expected a boolean for {} (as {} arg)", attr, arg_num(idx)) }
    )
  }
}

//
// floats
//


pub fn float_for_key(cx: &mut FunctionContext, obj: &Handle<JsObject>, attr:&str) -> Result<f32, Throw>{
  let key = cx.string(attr);
  match obj.get(cx, key)?.downcast::<JsNumber, _>(cx){
    Ok(num) => Ok(num.value(cx) as f32),
    Err(_e) => cx.throw_type_error(format!("Exptected a numerical value for \"{}\"", attr))
  }
}

pub fn floats_in(cx: &mut FunctionContext, vals: &[Handle<JsValue>]) -> Vec<f32>{
  let mut nums:Vec<f32> = Vec::new();
  for (i, val) in vals.iter().enumerate() {
    if let Ok(num) = val.downcast::<JsNumber, _>(cx){
      let val = num.value(cx) as f32;
      if val.is_finite() && !val.is_nan(){
        nums.push(val);
      }
    }
  }
  nums
}

pub fn opt_float_arg(cx: &mut FunctionContext, idx: usize) -> Option<f32>{
  match cx.argument_opt(idx as i32) {
    Some(arg) => match arg.downcast::<JsNumber, _>(cx) {
      Ok(v) => if v.value(cx).is_finite(){ Some(v.value(cx) as f32) }else{ None },
      Err(_e) => None
    },
    None => None
  }
}

pub fn float_arg_or(cx: &mut FunctionContext, idx: usize, default:f64) -> f32{
  match opt_float_arg(cx, idx){
    Some(v) => v,
    None => default as f32
  }
}

pub fn float_arg(cx: &mut FunctionContext, idx: usize, attr:&str) -> Result<f32, Throw>{
  let exists = cx.len() > idx as i32;
  match opt_float_arg(cx, idx){
    Some(v) => Ok(v),
    None => cx.throw_type_error(
      if exists { format!("{} must be a number", attr) }
      else { format!("Missing argument: expected a number for {} as {} arg", attr, arg_num(idx)) }
    )
  }
}

pub fn floats_to_array<'a>(cx: &mut FunctionContext<'a>, nums: &[f32]) -> JsResult<'a, JsValue> {
  let array = JsArray::new(cx, nums.len() as u32);
  for (i, val) in nums.iter().enumerate() {
    let num = cx.number(*val);
    array.set(cx, i as u32, num)?;
  }
  Ok(array.upcast())
}

//
// float spreads
//

pub fn opt_float_args(cx: &mut FunctionContext, rng: Range<usize>) -> Vec<f32>{
  let end = cmp::min(rng.end, cx.len() as usize);
  let rng = rng.start..end;

  let mut args:Vec<f32> = Vec::new();
  for i in rng.start..end{
    if let Ok(arg) = cx.argument::<JsValue>(i as i32){
      if let Ok(num) = arg.downcast::<JsNumber, _>(cx){
        args.push(num.value(cx) as f32);
      }
    }
  }
  args
}

pub fn float_args(cx: &mut FunctionContext, rng: Range<usize>) -> Result<Vec<f32>, Throw>{
  let need = rng.end - rng.start;
  let list = opt_float_args(cx, rng);
  let got = list.len();
  match got == need{
    true => Ok(list),
    false => cx.throw_type_error(format!("Not enough arguments: expected {} numbers (got {})", need, got))
  }
}

//
// Colors
//


pub fn css_to_color<'a>(cx: &mut FunctionContext<'a>, css:&str) -> Option<Color> {
  css.parse::<Rgba>().ok().map(|Rgba{red, green, blue, alpha}|
    Color::from_argb(
      (alpha*255.0).round() as u8,
      (red*255.0).round() as u8,
      (green*255.0).round() as u8,
      (blue*255.0).round() as u8,
    )
  )
}

pub fn color_in<'a>(cx: &mut FunctionContext<'a>, val: Handle<'a, JsValue>) -> Option<Color> {
  if val.is_a::<JsString, _>(cx) {
    let css = val.downcast::<JsString, _>(cx).unwrap().value(cx);
    return css_to_color(cx, &css)
  }

  if let Ok(obj) = val.downcast::<JsObject, _>(cx){
    if let Ok(attr) = obj.get(cx, "toString"){
      if let Ok(to_string) = attr.downcast::<JsFunction, _>(cx){
        let args: Vec<Handle<JsValue>> = vec![];
        if let Ok(result) = to_string.call(cx, obj, args){
          if let Ok(clr) = result.downcast::<JsString, _>(cx){
            let css = &clr.value(cx);
            return css_to_color(cx, css)
          }
        }
      }
    }
  }

  None
}

pub fn color_arg(cx: &mut FunctionContext, idx: usize) -> Option<Color> {
  match cx.argument_opt(idx as i32) {
    Some(arg) => color_in(cx, arg),
    _ => None
  }
}

pub fn color_to_css<'a>(cx: &mut FunctionContext<'a>, color:&Color) -> JsResult<'a, JsValue> {
  let RGB {r, g, b} = color.to_rgb();
  let css = match color.a() {
    255 => format!("#{:02x}{:02x}{:02x}", r, g, b),
    _ => {
      let alpha = format!("{:.3}", color.a() as f32 / 255.0);
      let alpha = alpha.trim_end_matches('0');
      format!("rgba({}, {}, {}, {})", r, g, b, if alpha=="0."{ "0" } else{ alpha })
    }
  };
  Ok(cx.string(css).upcast())
}

//
// Matrices
//

// pub fn matrix_in(cx: &mut FunctionContext, vals:&[Handle<JsValue>]) -> Result<Matrix, Throw>{
//   // for converting single js-array args
//   let terms = floats_in(vals);
//   match to_matrix(&terms){
//     Some(matrix) => Ok(matrix),
//     None => cx.throw_error(format!("expected 6 or 9 matrix values (got {})", terms.len()))
//   }
// }

pub fn to_matrix(t:&[f32]) -> Option<Matrix>{
  match t.len(){
    6 => Some(Matrix::new_all(t[0], t[1], t[2], t[3], t[4], t[5], 0.0, 0.0, 1.0)),
    9 => Some(Matrix::new_all(t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7], t[8])),
    _ => None
  }
}

// pub fn matrix_args(cx: &mut FunctionContext, rng: Range<usize>) -> Result<Matrix, Throw>{
//   // for converting inline args (e.g., in Path.transform())
//   let terms = opt_float_args(cx, rng);
//   match to_matrix(&terms){
//     Some(matrix) => Ok(matrix),
//     None => cx.throw_error(format!("expected 6 or 9 matrix values (got {})", terms.len()))
//   }
// }

pub fn opt_matrix_arg(cx: &mut FunctionContext, idx: usize) -> Option<Matrix>{
  if let Some(arg) = cx.argument_opt(idx as i32) {
    if let Ok(array) = arg.downcast::<JsArray, _>(cx) {
      if let Ok(vals) = array.to_vec(cx){
        let terms = floats_in(cx, &vals);
        return to_matrix(&terms)
      }
    }
  }
  None
}

pub fn matrix_arg(cx: &mut FunctionContext, idx:usize) -> Result<Matrix, Throw> {
  match opt_matrix_arg(cx, idx){
    Some(v) => Ok(v),
    None => cx.throw_type_error("expected a DOMMatrix")
  }
}

//
// Points
//

// pub fn points_in(vals:&[Handle<JsValue>]) -> Vec<Point>{
//   floats_in(&vals).as_slice()
//       .chunks(2)
//       .map(|pair| Point::new(pair[0], pair[1]))
//       .collect()
// }

//
// Path2D
//

use crate::path::{BoxedPath2D};

pub fn path2d_arg_opt(cx: &mut FunctionContext, idx:usize) -> Option<Path> {
  if let Some(arg) = cx.argument_opt(idx as i32){
    if let Ok(arg) = arg.downcast::<BoxedPath2D, _>(cx){
      let arg = arg.borrow();
      return Some(arg.path.clone())
    }
  }
  None
}


//
// Filters
//

pub enum FilterSpec{
  Plain{name:String, value:f32},
  Shadow{offset:Point, blur:f32, color:Color},
}

pub fn filter_arg(cx: &mut FunctionContext, idx: usize) -> Result<(String, Vec<FilterSpec>), Throw> {
  let arg = cx.argument::<JsObject>(idx as i32)?;
  let canonical = string_for_key(cx, &arg, "canonical")?;

  let obj = arg.get(cx, "filters")?.downcast_or_throw::<JsObject, _>(cx)?;
  let keys = obj.get_own_property_names(cx)?.to_vec(cx)?;
  let mut filters = vec![];
  for (name, key) in strings_in(cx, &keys).iter().zip(keys) {
    match name.as_str() {
      "drop-shadow" => {
        let values = obj.get(cx, key)?.downcast_or_throw::<JsArray, _>(cx)?;
        let nums = values.to_vec(cx)?;
        let dims = floats_in(cx, &nums);
        let color_str = values.get(cx, 3)?.downcast_or_throw::<JsString, _>(cx)?.value(cx);
        if let Some(color) = css_to_color(cx, &color_str) {
          filters.push(FilterSpec::Shadow{
            offset: Point::new(dims[0], dims[1]), blur: dims[2], color
          });
        }
      },
      _ => {
        let value = obj.get(cx, key)?.downcast_or_throw::<JsNumber, _>(cx)?.value(cx);
        filters.push(FilterSpec::Plain{
          name:name.to_string(), value:value as f32
        })
      }
    }
  }
  Ok( (canonical, filters) )
}

//
// Skia Enums
//

use skia_safe::{TileMode, TileMode::{Decal, Repeat}};
// pub fn to_tile_mode(mode_name:&str) -> Option<TileMode>{
//   let mode = match mode_name.to_lowercase().as_str(){
//     "clamp" => TileMode::Clamp,
//     "repeat" => TileMode::Repeat,
//     "mirror" => TileMode::Mirror,
//     "decal" => TileMode::Decal,
//     _ => return None
//   };
//   Some(mode)
// }

pub fn to_repeat_mode(repeat:&str) -> Option<(TileMode, TileMode)> {
  let mode = match repeat.to_lowercase().as_str() {
    "repeat" | "" => (Repeat, Repeat),
    "repeat-x" => (Repeat, Decal),
    "repeat-y" => (Decal, Repeat),
    "no-repeat" => (Decal, Decal),
    _ => return None
  };
  Some(mode)
}


use skia_safe::{FilterQuality};
pub fn to_filter_quality(mode_name:&str) -> Option<FilterQuality>{
  let mode = match mode_name.to_lowercase().as_str(){
    "low" => FilterQuality::Low,
    "medium" => FilterQuality::Medium,
    "high" => FilterQuality::High,
    _ => return None
  };
  Some(mode)
}

pub fn from_filter_quality(mode:FilterQuality) -> String{
  match mode{
    FilterQuality::Low => "low",
    FilterQuality::Medium => "medium",
    FilterQuality::High => "high",
    _ => "low"
  }.to_string()
}

use skia_safe::{PaintCap};
pub fn to_stroke_cap(mode_name:&str) -> Option<PaintCap>{
  let mode = match mode_name.to_lowercase().as_str(){
    "butt" => PaintCap::Butt,
    "round" => PaintCap::Round,
    "square" => PaintCap::Square,
        _ => return None
  };
  Some(mode)
}

pub fn from_stroke_cap(mode:PaintCap) -> String{
  match mode{
    PaintCap::Butt => "butt",
    PaintCap::Round => "round",
    PaintCap::Square => "square",
  }.to_string()
}

use skia_safe::{PaintJoin};
pub fn to_stroke_join(mode_name:&str) -> Option<PaintJoin>{
  let mode = match mode_name.to_lowercase().as_str(){
    "miter" => PaintJoin::Miter,
    "round" => PaintJoin::Round,
    "bevel" => PaintJoin::Bevel,
    _ => return None
  };
  Some(mode)
}

pub fn from_stroke_join(mode:PaintJoin) -> String{
  match mode{
    PaintJoin::Miter => "miter",
    PaintJoin::Round => "round",
    PaintJoin::Bevel => "bevel",
  }.to_string()
}


use skia_safe::{BlendMode};
pub fn to_blend_mode(mode_name:&str) -> Option<BlendMode>{
  let mode = match mode_name.to_lowercase().as_str(){
    "source-over" => BlendMode::SrcOver,
    "destination-over" => BlendMode::DstOver,
    "copy" => BlendMode::Src,
    "destination" => BlendMode::Dst,
    "clear" => BlendMode::Clear,
    "source-in" => BlendMode::SrcIn,
    "destination-in" => BlendMode::DstIn,
    "source-out" => BlendMode::SrcOut,
    "destination-out" => BlendMode::DstOut,
    "source-atop" => BlendMode::SrcATop,
    "destination-atop" => BlendMode::DstATop,
    "xor" => BlendMode::Xor,
    "lighter" => BlendMode::Plus,
    "multiply" => BlendMode::Multiply,
    "screen" => BlendMode::Screen,
    "overlay" => BlendMode::Overlay,
    "darken" => BlendMode::Darken,
    "lighten" => BlendMode::Lighten,
    "color-dodge" => BlendMode::ColorDodge,
    "color-burn" => BlendMode::ColorBurn,
    "hard-light" => BlendMode::HardLight,
    "soft-light" => BlendMode::SoftLight,
    "difference" => BlendMode::Difference,
    "exclusion" => BlendMode::Exclusion,
    "hue" => BlendMode::Hue,
    "saturation" => BlendMode::Saturation,
    "color" => BlendMode::Color,
    "luminosity" => BlendMode::Luminosity,
    _ => return None
  };
  Some(mode)
}

pub fn from_blend_mode(mode:BlendMode) -> String{
  match mode{
    BlendMode::SrcOver => "source-over",
    BlendMode::DstOver => "destination-over",
    BlendMode::Src => "copy",
    BlendMode::Dst => "destination",
    BlendMode::Clear => "clear",
    BlendMode::SrcIn => "source-in",
    BlendMode::DstIn => "destination-in",
    BlendMode::SrcOut => "source-out",
    BlendMode::DstOut => "destination-out",
    BlendMode::SrcATop => "source-atop",
    BlendMode::DstATop => "destination-atop",
    BlendMode::Xor => "xor",
    BlendMode::Plus => "lighter",
    BlendMode::Multiply => "multiply",
    BlendMode::Screen => "screen",
    BlendMode::Overlay => "overlay",
    BlendMode::Darken => "darken",
    BlendMode::Lighten => "lighten",
    BlendMode::ColorDodge => "color-dodge",
    BlendMode::ColorBurn => "color-burn",
    BlendMode::HardLight => "hard-light",
    BlendMode::SoftLight => "soft-light",
    BlendMode::Difference => "difference",
    BlendMode::Exclusion => "exclusion",
    BlendMode::Hue => "hue",
    BlendMode::Saturation => "saturation",
    BlendMode::Color => "color",
    BlendMode::Luminosity => "luminosity",
    _ => "source-over"
  }.to_string()
}

use skia_safe::{PathOp};
pub fn to_path_op(op_name:&str) -> Option<PathOp> {
  let op = match op_name.to_lowercase().as_str() {
    "difference" => PathOp::Difference,
    "intersect" => PathOp::Intersect,
    "union" => PathOp::Union,
    "xor" => PathOp::XOR,
    "reversedifference" | "complement" => PathOp::ReverseDifference,
    _ => return None
  };
  Some(op)
}


use skia_safe::path::FillType;

pub fn fill_rule_arg_or(cx: &mut FunctionContext, idx: usize, default: &str) -> Result<FillType, Throw>{
  let rule = match string_arg_or(cx, idx, default).as_str(){
    "nonzero" => FillType::Winding,
    "evenodd" => FillType::EvenOdd,
    _ => {
      let err_msg = format!("Argument {} ('fillRule') must be one of: \"nonzero\", \"evenodd\"", idx);
      return cx.throw_type_error(err_msg)
    }
  };
  Ok(rule)
}

// pub fn blend_mode_arg(cx: &mut FunctionContext, idx: usize, attr: &str) -> Result<BlendMode, Throw>{
//   let mode_name = string_arg(cx, idx, attr)?;
//   match to_blend_mode(&mode_name){
//     Some(blend_mode) => Ok(blend_mode),
//     None => cx.throw_error("blendMode must be SrcOver, DstOver, Src, Dst, Clear, SrcIn, DstIn, \
//                             SrcOut, DstOut, SrcATop, DstATop, Xor, Plus, Multiply, Screen, Overlay, \
//                             Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, \
//                             Exclusion, Hue, Saturation, Color, Luminosity, or Modulate")
//   }
// }


//
// Image Rects
//

pub fn fit_bounds(width: f32, height: f32, src: Rect, dst: Rect) -> (Rect, Rect) {
  let mut src = src;
  let mut dst = dst;
  let scale_x = dst.width() / src.width();
  let scale_y = dst.height() / src.height();

  if src.left < 0.0 {
    dst.left += -src.left * scale_x;
    src.left = 0.0;
  }

  if src.top < 0.0 {
    dst.top += -src.top * scale_y;
    src.top = 0.0;
  }

  if src.right > width{
    dst.right -= (src.right - width) * scale_x;
    src.right = width;
  }

  if src.bottom > height{
    dst.bottom -= (src.bottom - height) * scale_y;
    src.bottom = height;
  }

  (src, dst)
}

//
// Glutin KeyCodes
//

use glutin::event::{VirtualKeyCode};
pub fn from_key_code(code:VirtualKeyCode) -> String{
  match code{
    // The '1' key over the letters.
    VirtualKeyCode::Key1 => "1",
    // The '2' key over the letters.
    VirtualKeyCode::Key2 => "2",
    // The '3' key over the letters.
    VirtualKeyCode::Key3 => "3",
    // The '4' key over the letters.
    VirtualKeyCode::Key4 => "4",
    // The '5' key over the letters.
    VirtualKeyCode::Key5 => "5",
    // The '6' key over the letters.
    VirtualKeyCode::Key6 => "6",
    // The '7' key over the letters.
    VirtualKeyCode::Key7 => "7",
    // The '8' key over the letters.
    VirtualKeyCode::Key8 => "8",
    // The '9' key over the letters.
    VirtualKeyCode::Key9 => "9",
    // The '0' key over the 'O' and 'P' keys.
    VirtualKeyCode::Key0 => "0",

    VirtualKeyCode::A => "A",
    VirtualKeyCode::B => "B",
    VirtualKeyCode::C => "C",
    VirtualKeyCode::D => "D",
    VirtualKeyCode::E => "E",
    VirtualKeyCode::F => "F",
    VirtualKeyCode::G => "G",
    VirtualKeyCode::H => "H",
    VirtualKeyCode::I => "I",
    VirtualKeyCode::J => "J",
    VirtualKeyCode::K => "K",
    VirtualKeyCode::L => "L",
    VirtualKeyCode::M => "M",
    VirtualKeyCode::N => "N",
    VirtualKeyCode::O => "O",
    VirtualKeyCode::P => "P",
    VirtualKeyCode::Q => "Q",
    VirtualKeyCode::R => "R",
    VirtualKeyCode::S => "S",
    VirtualKeyCode::T => "T",
    VirtualKeyCode::U => "U",
    VirtualKeyCode::V => "V",
    VirtualKeyCode::W => "W",
    VirtualKeyCode::X => "X",
    VirtualKeyCode::Y => "Y",
    VirtualKeyCode::Z => "Z",

    // The Escape key, next to F1.
    VirtualKeyCode::Escape => "Escape",

    VirtualKeyCode::F1 => "F1",
    VirtualKeyCode::F2 => "F2",
    VirtualKeyCode::F3 => "F3",
    VirtualKeyCode::F4 => "F4",
    VirtualKeyCode::F5 => "F5",
    VirtualKeyCode::F6 => "F6",
    VirtualKeyCode::F7 => "F7",
    VirtualKeyCode::F8 => "F8",
    VirtualKeyCode::F9 => "F9",
    VirtualKeyCode::F10 => "F10",
    VirtualKeyCode::F11 => "F11",
    VirtualKeyCode::F12 => "F12",
    VirtualKeyCode::F13 => "F13",
    VirtualKeyCode::F14 => "F14",
    VirtualKeyCode::F15 => "F15",
    VirtualKeyCode::F16 => "F16",
    VirtualKeyCode::F17 => "F17",
    VirtualKeyCode::F18 => "F18",
    VirtualKeyCode::F19 => "F19",
    VirtualKeyCode::F20 => "F20",
    VirtualKeyCode::F21 => "F21",
    VirtualKeyCode::F22 => "F22",
    VirtualKeyCode::F23 => "F23",
    VirtualKeyCode::F24 => "F24",

    // Print Screen/SysRq.
    VirtualKeyCode::Snapshot => "Snapshot",
    // Scroll Lock.
    VirtualKeyCode::Scroll => "Scroll",
    // Pause/Break key, next to Scroll lock.
    VirtualKeyCode::Pause => "Pause",

    // `Insert`, next to Backspace.
    VirtualKeyCode::Insert => "Insert",
    VirtualKeyCode::Home => "Home",
    VirtualKeyCode::Delete => "Delete",
    VirtualKeyCode::End => "End",
    VirtualKeyCode::PageDown => "PageDown",
    VirtualKeyCode::PageUp => "PageUp",

    VirtualKeyCode::Left => "Left",
    VirtualKeyCode::Up => "Up",
    VirtualKeyCode::Right => "Right",
    VirtualKeyCode::Down => "Down",

    // The Backspace key, right over Enter.
    VirtualKeyCode::Back => "Backspace",
    // The Enter key.
    VirtualKeyCode::Return => "Return",
    // The space bar.
    VirtualKeyCode::Space => "Space",

    // The "Compose" key on Linux.
    VirtualKeyCode::Compose => "Compose",

    VirtualKeyCode::Caret => "Caret",

    VirtualKeyCode::Numlock => "Numlock",
    VirtualKeyCode::Numpad0 => "Numpad0",
    VirtualKeyCode::Numpad1 => "Numpad1",
    VirtualKeyCode::Numpad2 => "Numpad2",
    VirtualKeyCode::Numpad3 => "Numpad3",
    VirtualKeyCode::Numpad4 => "Numpad4",
    VirtualKeyCode::Numpad5 => "Numpad5",
    VirtualKeyCode::Numpad6 => "Numpad6",
    VirtualKeyCode::Numpad7 => "Numpad7",
    VirtualKeyCode::Numpad8 => "Numpad8",
    VirtualKeyCode::Numpad9 => "Numpad9",
    VirtualKeyCode::NumpadAdd => "NumpadAdd",
    VirtualKeyCode::NumpadDivide => "NumpadDivide",
    VirtualKeyCode::NumpadDecimal => "NumpadDecimal",
    VirtualKeyCode::NumpadComma => "NumpadComma",
    VirtualKeyCode::NumpadEnter => "NumpadEnter",
    VirtualKeyCode::NumpadEquals => "NumpadEquals",
    VirtualKeyCode::NumpadMultiply => "NumpadMultiply",
    VirtualKeyCode::NumpadSubtract => "NumpadSubtract",

    VirtualKeyCode::AbntC1 => "AbntC1",
    VirtualKeyCode::AbntC2 => "AbntC2",
    VirtualKeyCode::Apostrophe => "Apostrophe",
    VirtualKeyCode::Apps => "Apps",
    VirtualKeyCode::Asterisk => "Asterisk",
    VirtualKeyCode::At => "At",
    VirtualKeyCode::Ax => "Ax",
    VirtualKeyCode::Backslash => "Backslash",
    VirtualKeyCode::Calculator => "Calculator",
    VirtualKeyCode::Capital => "Capital",
    VirtualKeyCode::Colon => "Colon",
    VirtualKeyCode::Comma => "Comma",
    VirtualKeyCode::Convert => "Convert",
    VirtualKeyCode::Equals => "Equals",
    VirtualKeyCode::Grave => "Grave",
    VirtualKeyCode::Kana => "Kana",
    VirtualKeyCode::Kanji => "Kanji",
    VirtualKeyCode::LAlt => "LAlt",
    VirtualKeyCode::LBracket => "LBracket",
    VirtualKeyCode::LControl => "LControl",
    VirtualKeyCode::LShift => "LShift",
    VirtualKeyCode::LWin => "LMeta",
    VirtualKeyCode::Mail => "Mail",
    VirtualKeyCode::MediaSelect => "MediaSelect",
    VirtualKeyCode::MediaStop => "MediaStop",
    VirtualKeyCode::Minus => "Minus",
    VirtualKeyCode::Mute => "Mute",
    VirtualKeyCode::MyComputer => "MyComputer",
    // also called "Next"
    VirtualKeyCode::NavigateForward => "NavigateForward",
    // also called "Prior"
    VirtualKeyCode::NavigateBackward => "NavigateBackward",
    VirtualKeyCode::NextTrack => "NextTrack",
    VirtualKeyCode::NoConvert => "NoConvert",
    VirtualKeyCode::OEM102 => "OEM102",
    VirtualKeyCode::Period => "Period",
    VirtualKeyCode::PlayPause => "PlayPause",
    VirtualKeyCode::Plus => "Plus",
    VirtualKeyCode::Power => "Power",
    VirtualKeyCode::PrevTrack => "PrevTrack",
    VirtualKeyCode::RAlt => "RAlt",
    VirtualKeyCode::RBracket => "RBracket",
    VirtualKeyCode::RControl => "RControl",
    VirtualKeyCode::RShift => "RShift",
    VirtualKeyCode::RWin => "RMeta",
    VirtualKeyCode::Semicolon => "Semicolon",
    VirtualKeyCode::Slash => "Slash",
    VirtualKeyCode::Sleep => "Sleep",
    VirtualKeyCode::Stop => "Stop",
    VirtualKeyCode::Sysrq => "Sysrq",
    VirtualKeyCode::Tab => "Tab",
    VirtualKeyCode::Underline => "Underline",
    VirtualKeyCode::Unlabeled => "Unlabeled",
    VirtualKeyCode::VolumeDown => "VolumeDown",
    VirtualKeyCode::VolumeUp => "VolumeUp",
    VirtualKeyCode::Wake => "Wake",
    VirtualKeyCode::WebBack => "WebBack",
    VirtualKeyCode::WebFavorites => "WebFavorites",
    VirtualKeyCode::WebForward => "WebForward",
    VirtualKeyCode::WebHome => "WebHome",
    VirtualKeyCode::WebRefresh => "WebRefresh",
    VirtualKeyCode::WebSearch => "WebSearch",
    VirtualKeyCode::WebStop => "WebStop",
    VirtualKeyCode::Yen => "Yen",
    VirtualKeyCode::Copy => "Copy",
    VirtualKeyCode::Paste => "Paste",
    VirtualKeyCode::Cut => "Cut",
  }.to_string()
}

use glutin::window::CursorIcon;
pub fn to_cursor_icon(cursor_name:&str) -> Option<CursorIcon> {
  let cursor = match cursor_name.to_lowercase().as_str() {
    "default" => CursorIcon::Default,
    "crosshair" => CursorIcon::Crosshair,
    "hand" => CursorIcon::Hand,
    "arrow" => CursorIcon::Arrow,
    "move" => CursorIcon::Move,
    "text" => CursorIcon::Text,
    "wait" => CursorIcon::Wait,
    "help" => CursorIcon::Help,
    "progress" => CursorIcon::Progress,
    "not-allowed" => CursorIcon::NotAllowed,
    "context-menu" => CursorIcon::ContextMenu,
    "cell" => CursorIcon::Cell,
    "vertical-text" => CursorIcon::VerticalText,
    "alias" => CursorIcon::Alias,
    "copy" => CursorIcon::Copy,
    "no-drop" => CursorIcon::NoDrop,
    "grab" => CursorIcon::Grab,
    "grabbing" => CursorIcon::Grabbing,
    "all-scroll" => CursorIcon::AllScroll,
    "zoom-in" => CursorIcon::ZoomIn,
    "zoom-out" => CursorIcon::ZoomOut,
    "e-resize" => CursorIcon::EResize,
    "n-resize" => CursorIcon::NResize,
    "ne-resize" => CursorIcon::NeResize,
    "nw-resize" => CursorIcon::NwResize,
    "s-resize" => CursorIcon::SResize,
    "se-resize" => CursorIcon::SeResize,
    "sw-resize" => CursorIcon::SwResize,
    "w-resize" => CursorIcon::WResize,
    "ew-resize" => CursorIcon::EwResize,
    "ns-resize" => CursorIcon::NsResize,
    "nesw-resize" => CursorIcon::NeswResize,
    "nwse-resize" => CursorIcon::NwseResize,
    "col-resize" => CursorIcon::ColResize,
    "row-resize" => CursorIcon::RowResize,
    _ => return None
  };
  Some(cursor)
}

use crate::display::Fit;
pub fn to_canvas_fit(fit_name:&str) -> Option<Fit> {
  let fit = match fit_name.to_lowercase().as_str() {
    "contain" => Fit::Contain{x:true, y:true},
    "contain-x" => Fit::Contain{x:true, y:false},
    "contain-y" => Fit::Contain{x:false, y:true},
    "cover" => Fit::Cover,
    "fill" => Fit::Fill,
    "scale-down" => Fit::ScaleDown,
    _ => return None
  };
  Some(fit)
}

//
// PDF creation
//

use skia_safe::{pdf, Document};

pub fn pdf_document(quality:f32, density:f32) -> Document{
  let mut meta = pdf::Metadata::default();
  meta.producer = "Skia Canvas <https://github.com/samizdatco/skia-canvas>".to_string();
  meta.encoding_quality = Some((quality*100.0) as i32);
  meta.raster_dpi = Some(density * 72.0);
  pdf::new_document(Some(&meta))
}
