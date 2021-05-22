use std::collections::HashMap;
use neon::prelude::*;
use glutin::dpi::{LogicalSize, LogicalPosition};
use glutin::event::{KeyboardInput, VirtualKeyCode, WindowEvent, ModifiersState,
                    ElementState, MouseButton, MouseScrollDelta};

use crate::utils::from_key_code;

enum StateChange{
  Keyboard{event:String, key:String, code:u32, repeat:bool},
  Input(char),
  Mouse(String),
  Wheel(LogicalPosition<f64>),
  Position(LogicalPosition<i32>),
  Size(LogicalSize<u32>),
  Fullscreen(bool),
}

pub struct EventQueue{
  changes: Vec<StateChange>,
  key_modifiers: ModifiersState,
  key_repeats: HashMap<VirtualKeyCode, i32>,
  mouse_point: LogicalPosition::<i32>,
  mouse_button: Option<u16>,
}

impl EventQueue {
  pub fn new() -> Self {
    EventQueue{
      changes: vec![],
      key_modifiers: ModifiersState::empty(),
      key_repeats: HashMap::new(),
      mouse_point: LogicalPosition::<i32>{x:0, y:0},
      mouse_button: None,
    }
  }

  pub fn is_empty(&self) -> bool{
    self.changes.len() == 0
  }

  pub fn went_fullscreen(&mut self, did_go_fullscreen:bool){
    self.changes.push(StateChange::Fullscreen(did_go_fullscreen));
    self.key_repeats.clear(); // keyups don't get delivered during the transition apparently?
  }

  pub fn capture(&mut self, event:&WindowEvent, dpr:f64){
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

      WindowEvent::KeyboardInput { input:
        KeyboardInput { scancode, state, virtual_keycode: Some(keycode), ..}, ..
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

  pub fn digest<'a>(&mut self, cx: &mut FunctionContext<'a>) -> Vec<Handle<'a, JsValue>>{

    let mut payload:Vec<Handle<JsValue>> = (0..17).map(|_|
      //   0–5: x, y, w, h, fullscreen, [alt, ctrl, meta, shift]
      //  6–10: input, keyEvent, key, code, repeat,
      // 11–14: [mouseEvents], mouseX, mouseY, button,
      // 15–16: wheelX, wheelY
      cx.undefined().upcast::<JsValue>()
    ).collect();

    let mut include_mods = false;
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
          include_mods = true;
          payload[6] = cx.string(character.to_string()).upcast::<JsValue>(); // input
        }
        StateChange::Keyboard{event, key, code, repeat} => {
          include_mods = true;
          payload[7] = cx.string(event).upcast::<JsValue>();     // keyup | keydown
          payload[8] = cx.string(key).upcast::<JsValue>();       // key
          payload[9] = cx.number(*code).upcast::<JsValue>();     // code
          payload[10] = cx.boolean(*repeat).upcast::<JsValue>(); // repeat
        }
        StateChange::Mouse(event_type) => {
          include_mods = true;
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
        payload[14] = cx.number(button_id).upcast::<JsValue>(); // button
        self.mouse_button = None;
      }
    }

    if include_mods{
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

