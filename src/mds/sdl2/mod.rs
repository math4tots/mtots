use crate::anyhow::Result;
use crate::Eval;
use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::NativeFunction;
use crate::RcStr;
use crate::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

mod bindings;
mod keys;
mod state;

use bindings::*;
use keys::keycode_to_key;
use keys::KEY_COUNT;
use state::SdlError;
use state::State;

pub const NAME: &str = "a._sdl2";

fn get(global: &mut Globals) -> Rc<RefCell<State>> {
    global.get_from_stash()
}

pub(super) fn load(globals: &mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> {
    let sr = globals.symbol_registry();
    let mut map = HashMap::<RcStr, Value>::new();

    map.extend(
        vec![
            NativeFunction::sdnew0(sr, "init", &[], None, |globals, _args, _kwargs| {
                let state = get(globals);
                try_(globals, state.borrow_mut().sdl())?;
                Ok(Value::Nil)
            }),
            NativeFunction::sdnew0(
                sr,
                "new_window",
                &["title", "width", "height", "fullscreen"],
                None,
                |globals, args, _kwargs| {
                    let title = Eval::expect_string(globals, &args[0])?;
                    let width = Eval::expect_u32(globals, &args[1])?;
                    let height = Eval::expect_u32(globals, &args[2])?;
                    let fullscreen = Eval::truthy(globals, &args[3])?;
                    let state = get(globals);
                    let window = try_(
                        globals,
                        state
                            .borrow_mut()
                            .new_window(title, width, height, fullscreen),
                    )?;
                    Ok(from_window(window))
                },
            ),
            NativeFunction::sdnew0(
                sr,
                "window_to_canvas",
                &["window"],
                None,
                |globals, args, _kwargs| {
                    let window = move_window(globals, &args[0])?;
                    let canvas = try_(globals, window.into_canvas().build())?;
                    Ok(from_canvas(canvas))
                },
            ),
            NativeFunction::sdnew0(
                sr,
                "canvas_set_draw_color",
                &["canvas", "color"],
                None,
                |globals, args, _kwargs| {
                    let mut canvas = to_canvas_mut(globals, &args[0])?;
                    let color = to_color(globals, &args[1])?;
                    canvas.set_draw_color(color);
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::sdnew0(
                sr,
                "canvas_clear",
                &["canvas"],
                None,
                |globals, args, _kwargs| {
                    let mut canvas = to_canvas_mut(globals, &args[0])?;
                    canvas.clear();
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::sdnew0(
                sr,
                "canvas_present",
                &["canvas"],
                None,
                |globals, args, _kwargs| {
                    let mut canvas = to_canvas_mut(globals, &args[0])?;
                    canvas.present();
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::sdnew0(
                sr,
                "canvas_size",
                &["canvas"],
                None,
                |globals, args, _kwargs| {
                    let canvas = to_canvas(globals, &args[0])?;
                    let (width, height) = canvas.window().size();
                    Ok(vec![
                        Value::Int(width as i64),
                        Value::Int(height as i64),
                    ].into())
                },
            ),
            NativeFunction::sdnew0(
                sr,
                "canvas_set_size",
                &["canvas", "width", "height"],
                None,
                |globals, args, _kwargs| {
                    let mut canvas = to_canvas_mut(globals, &args[0])?;
                    let width = Eval::expect_u32(globals, &args[1])?;
                    let height = Eval::expect_u32(globals, &args[2])?;
                    try_(globals, canvas.window_mut().set_size(width, height))?;
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::sdnew0(
                sr,
                "canvas_fill_rect",
                &["canvas", "rect"],
                None,
                |globals, args, _kwargs| {
                    let mut canvas = to_canvas_mut(globals, &args[0])?;
                    let rect = to_rect(globals, &args[1])?;
                    try_(globals, canvas.fill_rect(rect))?;
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::sdnew0(sr, "poll", &[], None, |globals, _args, _kwargs| {
                let state = get(globals);
                let mut state = state.borrow_mut();
                let event_pump = try_(globals, state.event_pump())?;
                let events: Vec<_> = event_pump.poll_iter().collect();
                let events = from_events(globals, events)?;
                Ok(events.into())
            }),
        ]
        .into_iter()
        .map(|f| (f.name().clone(), f.into())),
    );

    Ok({
        let mut ret = HMap::new();
        for (key, value) in map {
            ret.insert(key, Rc::new(RefCell::new(value)));
        }
        ret
    })
}

fn try_<T, E: Into<SdlError>>(globals: &mut Globals, r: Result<T, E>) -> EvalResult<T> {
    match r {
        Ok(x) => Ok(x),
        Err(error) => globals.set_exc_str(&format!("{:?}", error.into())),
    }
}
