use crate::ErrorIndicator;
use crate::Eval;
use crate::EvalResult;
use crate::Globals;
use crate::HMap;
use crate::NativeFunction;
use crate::Opaque;
use crate::RcStr;
use crate::Symbol;
use crate::Value;
use ggez::event;
use ggez::event::Axis;
use ggez::event::Button;
use ggez::event::EventHandler;
use ggez::event::GamepadId;
use ggez::event::KeyCode;
use ggez::event::KeyMods;
use ggez::event::MouseButton;
use ggez::graphics;
use ggez::graphics::spritebatch::SpriteBatch;
use ggez::graphics::spritebatch::SpriteIdx;
use ggez::graphics::Canvas;
use ggez::graphics::Color;
use ggez::graphics::Image;
use ggez::graphics::Mesh;
use ggez::graphics::MeshBuilder;
use ggez::graphics::Scale;
use ggez::graphics::Text;
use ggez::graphics::TextFragment;
use ggez::Context;
use ggez::ContextBuilder;
use ggez::GameError;
use ggez::GameResult;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::rc::Rc;

pub const NAME: &str = "a._ggez";

type Point = ggez::mint::Point2<f32>;

fn mkpt(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn try_<R>(globals: &mut Globals, r: GameResult<R>) -> EvalResult<R> {
    match r {
        Ok(r) => Ok(r),
        Err(error) => globals.set_exc_str(&format!("{:?}", error)),
    }
}

pub(super) fn load(globals: &mut Globals) -> EvalResult<HMap<RcStr, Rc<RefCell<Value>>>> {
    let sr = globals.symbol_registry();
    let mut map = HashMap::<RcStr, Value>::new();

    map.extend(
        vec![
            NativeFunction::simple0(
                sr,
                "new_color",
                &["r", "g", "b", "a"],
                |globals, args, _kwargs| {
                    let r = Eval::expect_floatlike(globals, &args[0])? as f32;
                    let g = Eval::expect_floatlike(globals, &args[1])? as f32;
                    let b = Eval::expect_floatlike(globals, &args[2])? as f32;
                    let a = Eval::expect_floatlike(globals, &args[3])? as f32;
                    from_color(globals, Color { r, g, b, a })
                },
            ),
            NativeFunction::simple0(
                sr,
                "new_text_simple",
                &["text", "color", "scale"],
                |globals, args, _kwargs| {
                    let text = Eval::expect_string(globals, &args[0])?;
                    let color = if let Value::Nil = &args[1] {
                        None
                    } else {
                        Some(to_color_ref(globals, &args[1])?.clone())
                    };
                    let scale = if let Value::Nil = &args[2] {
                        None
                    } else {
                        let factor = Eval::expect_floatlike(globals, &args[2])? as f32;
                        Some(Scale {
                            x: factor,
                            y: factor,
                        })
                    };
                    let fragment = TextFragment {
                        text: text.str().to_owned(),
                        color,
                        font: None,
                        scale,
                    };
                    let text = Text::new(fragment);
                    from_text(globals, text)
                },
            ),
            NativeFunction::simple0(
                sr,
                "text_width",
                &["text", "ctx"],
                |globals, args, _kwargs| {
                    let text = to_text(globals, &args[0])?;
                    let ctx_refcell = to_ctx(globals, &args[1])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    Ok((text.width(ctx.get_mut()) as f64).into())
                },
            ),
            NativeFunction::simple0(
                sr,
                "text_height",
                &["text", "ctx"],
                |globals, args, _kwargs| {
                    let text = to_text(globals, &args[0])?;
                    let ctx_refcell = to_ctx(globals, &args[1])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    Ok((text.height(ctx.get_mut()) as f64).into())
                },
            ),
            NativeFunction::simple0(sr, "new_mesh_builder", &[], |globals, _args, _kwargs| {
                from_mesh_builder(globals, MeshBuilder::new())
            }),
            NativeFunction::simple0(
                sr,
                "mesh_builder_circle",
                &["mesh_builder", "center", "radius", "tolerance", "color"],
                |globals, args, _kwargs| {
                    let mesh_builder = to_mesh_builder_ref(globals, &args[0])?;
                    let center = expect_point(globals, &args[1])?;
                    let radius = Eval::expect_floatlike(globals, &args[2])? as f32;
                    let tolerance = Eval::expect_floatlike(globals, &args[3])? as f32;
                    let color = to_color_ref(globals, &args[4])?.clone();
                    mesh_builder.borrow_mut().circle(
                        graphics::DrawMode::fill(),
                        center,
                        radius,
                        tolerance,
                        color,
                    );
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::simple0(
                sr,
                "mesh_builder_polygon",
                &["mesh_builder", "points", "color"],
                |globals, args, _kwargs| {
                    let mesh_builder = to_mesh_builder_ref(globals, &args[0])?;
                    let points = {
                        let mut points = Vec::new();
                        for point in Eval::iterable_to_vec(globals, &args[1])? {
                            points.push(expect_point(globals, &point)?);
                        }
                        points
                    };
                    let color = to_color_ref(globals, &args[2])?.clone();
                    try_(
                        globals,
                        mesh_builder.borrow_mut().polygon(
                            graphics::DrawMode::fill(),
                            &points,
                            color,
                        ),
                    )?;
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::simple0(
                sr,
                "mesh_builder_build",
                &["mesh_builder", "ctx"],
                |globals, args, _kwargs| {
                    let mesh_builder = to_mesh_builder(globals, &args[0])?;
                    let ctx_refcell = to_ctx(globals, &args[1])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    let mesh = try_(globals, mesh_builder.build(ctx.get_mut()))?;
                    from_mesh(globals, mesh)
                },
            ),
            NativeFunction::simple0(
                sr,
                "new_image_from_colors",
                &["ctx", "width", "height", "colors"],
                |globals, args, _kwargs| {
                    let ctx_refcell = to_ctx(globals, &args[0])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    let width = Eval::expect_usize(globals, &args[1])? as u16;
                    let height = Eval::expect_usize(globals, &args[2])? as u16;
                    let iterator = Eval::iter(globals, &args[3])?;
                    let mut colors = Vec::new();
                    while let Some(color) = Eval::next(globals, &iterator)? {
                        let color = to_color_ref(globals, &color)?;
                        let (r, g, b, a) = color.to_rgba();
                        colors.extend(&[r, g, b, a]);
                    }
                    let image = try_(
                        globals,
                        Image::from_rgba8(ctx.get_mut(), width, height, &colors),
                    )?;
                    from_image(globals, image)
                },
            ),
            NativeFunction::sdnew(
                sr,
                "new_image_from_rect",
                (&["ctx", "width", "height", "color"], &[], None, None),
                Some("Convenience function for creating a rectangle image with a solid color"),
                |globals, args, _kwargs| {
                    let ctx_refcell = to_ctx(globals, &args[0])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    let width = Eval::expect_usize(globals, &args[1])?;
                    let height = Eval::expect_usize(globals, &args[2])?;
                    let (r, g, b, a) = to_color_ref(globals, &args[3])?.clone().into();
                    let len = width * height;
                    let mut buffer = Vec::with_capacity(len);
                    for _ in 0..len {
                        buffer.extend(&[r, g, b, a]);
                    }
                    let width = Eval::check_u16(globals, width as i64)?;
                    let height = Eval::check_u16(globals, height as i64)?;
                    let image = try_(
                        globals,
                        Image::from_rgba8(ctx.get_mut(), width, height, &buffer),
                    )?;
                    from_image(globals, image)
                },
            ),
            NativeFunction::simple0(
                sr,
                "new_sprite_batch_from_image",
                &["image"],
                |globals, args, _kwargs| {
                    let image = to_image(globals, &args[0])?;
                    let sprite_batch = SpriteBatch::new(image.clone());
                    from_sprite_batch(globals, sprite_batch)
                },
            ),
            NativeFunction::simple0(
                sr,
                "sprite_batch_add",
                &["sprite_batch", "destination", "rotation", "scale", "offset"],
                |globals, args, _kwargs| {
                    let mut sprite_batch = to_sprite_batch_mut(globals, &args[0])?;
                    let destination = expect_point(globals, &args[1])?;
                    let rotation = Eval::expect_floatlike(globals, &args[2])? as f32;
                    let (scale_x, scale_y) = expect_f32_pair(globals, &args[3])?;
                    let (offset_x, offset_y) = expect_f32_pair(globals, &args[4])?;
                    let params = graphics::DrawParam::default()
                        .dest(destination)
                        .rotation(rotation)
                        .scale([scale_x, scale_y])
                        .offset([offset_x, offset_y]);
                    let sprite_id = sprite_batch.add(params);
                    from_spriteidx(globals, sprite_id)
                },
            ),
            NativeFunction::simple0(
                sr,
                "sprite_batch_set",
                &[
                    "sprite_batch",
                    "spriteidx",
                    "destination",
                    "rotation",
                    "scale",
                    "offset",
                ],
                |globals, args, _kwargs| {
                    let mut sprite_batch = to_sprite_batch_mut(globals, &args[0])?;
                    let sprite_id = copy_spriteidx(globals, &args[1])?;
                    let destination = expect_point(globals, &args[2])?;
                    let rotation = Eval::expect_floatlike(globals, &args[3])? as f32;
                    let (scale_x, scale_y) = expect_f32_pair(globals, &args[4])?;
                    let (offset_x, offset_y) = expect_f32_pair(globals, &args[5])?;
                    let params = graphics::DrawParam::default()
                        .dest(destination)
                        .rotation(rotation)
                        .scale([scale_x, scale_y])
                        .offset([offset_x, offset_y]);
                    try_(globals, sprite_batch.set(sprite_id, params))?;
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::simple0(
                sr,
                "sprite_batch_clear",
                &["sprite_batch"],
                |globals, args, _kwargs| {
                    let mut sprite_batch = to_sprite_batch_mut(globals, &args[0])?;
                    sprite_batch.clear();
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::simple0(
                sr,
                "ctx_draw",
                &[
                    "ctx",
                    "drawable",
                    "destination",
                    "rotation",
                    "scale",
                    "offset",
                ],
                |globals, args, _kwargs| {
                    let ctx_refcell = to_ctx(globals, &args[0])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    let drawable = to_drawable(globals, &args[1])?;
                    let destination = expect_point(globals, &args[2])?;
                    let rotation = Eval::expect_floatlike(globals, &args[3])? as f32;
                    let (scale_x, scale_y) = expect_f32_pair(globals, &args[4])?;
                    let (offset_x, offset_y) = expect_f32_pair(globals, &args[5])?;
                    let params = graphics::DrawParam::default()
                        .dest(destination)
                        .rotation(rotation)
                        .scale([scale_x, scale_y])
                        .offset([offset_x, offset_y]);
                    try_(globals, draw(ctx.get_mut(), &drawable, params))?;
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::sdnew(
                sr,
                "ctx_drawables_to_image",
                (
                    &["ctx", "drawables_with_params", "width", "height", "samples"],
                    &[],
                    None,
                    None,
                ),
                Some(concat!(
                    "Accepts a List of drawables_with_params, width and height of the image, ",
                    "and the number of samples for anti-aliasing, and returns ",
                    "an image\n",
                    "The params are expected to be provided as a 4-tuple of ",
                    "[scale, rotation, x, y]\n",
                    "NOTE: I've never actually managed to get this to work... ",
                    "It seems to panic when I actually try to use it\n"
                )),
                |globals, args, _kwargs| {
                    let ctx_refcell = to_ctx(globals, &args[0])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    let ctx = ctx.get_mut();
                    let drawables_with_params = Eval::expect_list(globals, &args[1])?;
                    let width = expect_u16(globals, &args[2])?;
                    let height = expect_u16(globals, &args[3])?;
                    let samples = expect_u32(globals, &args[4])?;
                    let samples = match ggez::conf::NumSamples::from_u32(samples) {
                        Some(samples) => samples,
                        None => {
                            return globals
                                .set_exc_str(&format!("Invalid sample count ({})", samples,))
                        }
                    };

                    fn draw_all(
                        ctx: &mut Context,
                        globals: &mut Globals,
                        drawables_with_params: &Vec<Value>,
                    ) -> EvalResult<()> {
                        for drawable_with_params in drawables_with_params {
                            let (drawable, params) =
                                Eval::unpack_pair(globals, drawable_with_params)?;
                            let drawable = to_drawable(globals, &drawable)?;
                            let (scale, rot, x, y) = expect_4_f32(globals, &params)?;
                            try_(
                                globals,
                                draw(
                                    ctx,
                                    &drawable,
                                    graphics::DrawParam::default()
                                        .offset([0.5, 0.5])
                                        .rotation(rot)
                                        .scale([scale, scale])
                                        .dest([x, y]),
                                ),
                            )?;
                        }
                        Ok(())
                    }

                    let canvas = try_(globals, Canvas::new(ctx, width, height, samples))?;
                    ggez::graphics::set_canvas(ctx, Some(&canvas));
                    let r = draw_all(ctx, globals, drawables_with_params);
                    ggez::graphics::set_canvas(ctx, None);
                    r?;

                    from_image(globals, canvas.into_inner())
                },
            ),
            NativeFunction::simple0(sr, "ctx_size", &["ctx"], |globals, args, _kwargs| {
                let ctx_refcell = to_ctx(globals, &args[0])?;
                let ctx = ctx_refcell.borrow();
                let (width, height) = graphics::drawable_size(ctx.get());
                Ok(vec![Value::Float(width as f64), Value::Float(height as f64)].into())
            }),
            NativeFunction::simple0(sr, "ctx_fps", &["ctx"], |globals, args, _kwargs| {
                let ctx_refcell = to_ctx(globals, &args[0])?;
                let ctx = ctx_refcell.borrow();
                Ok(ggez::timer::fps(ctx.get()).into())
            }),
            NativeFunction::simple0(sr, "ctx_quit", &["ctx"], |globals, args, _kwargs| {
                let ctx_refcell = to_ctx(globals, &args[0])?;
                let mut ctx = ctx_refcell.borrow_mut();
                event::quit(ctx.get_mut());
                Ok(Value::Nil)
            }),
            NativeFunction::simple0(
                sr,
                "ctx_set_drawable_size",
                &["ctx", "w", "h"],
                |globals, args, _kwargs| {
                    let ctx_refcell = to_ctx(globals, &args[0])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    let w = Eval::expect_floatlike(globals, &args[1])? as f32;
                    let h = Eval::expect_floatlike(globals, &args[2])? as f32;
                    try_(
                        globals,
                        ggez::graphics::set_drawable_size(ctx.get_mut(), w, h),
                    )?;
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::simple0(
                sr,
                "ctx_set_screen_coordinates",
                &["ctx", "x", "y", "w", "h"],
                |globals, args, _kwargs| {
                    let ctx_refcell = to_ctx(globals, &args[0])?;
                    let mut ctx = ctx_refcell.borrow_mut();
                    let x = Eval::expect_floatlike(globals, &args[1])? as f32;
                    let y = Eval::expect_floatlike(globals, &args[2])? as f32;
                    let w = Eval::expect_floatlike(globals, &args[3])? as f32;
                    let h = Eval::expect_floatlike(globals, &args[4])? as f32;
                    let rect = ggez::graphics::Rect { x, y, w, h };
                    try_(
                        globals,
                        ggez::graphics::set_screen_coordinates(ctx.get_mut(), rect),
                    )?;
                    Ok(Value::Nil)
                },
            ),
            NativeFunction::simple0(sr, "get_all_keycodes", &[], |globals, _args, _kwargs| {
                let keycodes = list_keycode_symbols(globals);
                Ok(keycodes.into())
            }),
            NativeFunction::simple0(
                sr,
                "start",
                &[
                    "context_class",
                    "name",
                    "author",
                    "sleep_per_frame",
                    "update",
                    "draw",
                    "mouse_down",
                    "key_down",
                    "key_up",
                    "text_input",
                    "gamepad_button_down",
                    "gamepad_axis",
                ],
                |globals, args, _kwargs| {
                    struct State<'a> {
                        globals: &'a mut Globals,
                        context_class: &'a Value,
                        sleep_per_frame: Option<std::time::Duration>,
                        update: Rc<RefCell<Value>>,
                        draw: Rc<RefCell<Value>>,
                        mouse_down: Rc<RefCell<Value>>,
                        key_down: Rc<RefCell<Value>>,
                        key_up: Rc<RefCell<Value>>,
                        text_input: Rc<RefCell<Value>>,
                        gamepad_button_down: Rc<RefCell<Value>>,
                        gamepad_axis: Rc<RefCell<Value>>,

                        keycode_list: Vec<Value>,
                        gamepad_button_map: HashMap<Button, Value>,
                        gamepad_axis_map: HashMap<Axis, Value>,
                        gamepad_id_list: Vec<GamepadId>,
                        gamepad_id_map: HashMap<GamepadId, usize>,

                        symbol_shift: Symbol,
                        symbol_ctrl: Symbol,
                        symbol_alt: Symbol,
                        symbol_logo: Symbol,
                        symbol_repeat: Symbol,

                        symbol_left: Symbol,
                        symbol_right: Symbol,
                        symbol_middle: Symbol,
                    }

                    impl<'a> State<'a> {
                        fn new(
                            globals: &'a mut Globals,
                            _ctx: &mut Context,
                            context_class: &'a Value,
                            sleep_per_frame: Option<std::time::Duration>,
                            update: Rc<RefCell<Value>>,
                            draw: Rc<RefCell<Value>>,
                            mouse_down: Rc<RefCell<Value>>,
                            key_down: Rc<RefCell<Value>>,
                            key_up: Rc<RefCell<Value>>,
                            text_input: Rc<RefCell<Value>>,
                            gamepad_button_down: Rc<RefCell<Value>>,
                            gamepad_axis: Rc<RefCell<Value>>,
                        ) -> State<'a> {
                            let keycode_list = list_keycode_symbols(globals);
                            let gamepad_button_map = list_gamepad_buttons(globals);
                            let gamepad_axis_map = list_gamepad_axes(globals);
                            let symbol_shift = globals.intern_str("shift");
                            let symbol_ctrl = globals.intern_str("ctrl");
                            let symbol_alt = globals.intern_str("alt");
                            let symbol_logo = globals.intern_str("logo");
                            let symbol_repeat = globals.intern_str("repeat");
                            let symbol_left = globals.intern_str("left");
                            let symbol_right = globals.intern_str("right");
                            let symbol_middle = globals.intern_str("middle");
                            State {
                                globals,
                                context_class,
                                sleep_per_frame,
                                update,
                                draw,
                                mouse_down,
                                key_down,
                                key_up,
                                text_input,
                                gamepad_button_down,
                                gamepad_axis,
                                keycode_list,
                                gamepad_button_map,
                                gamepad_axis_map,
                                gamepad_id_list: Vec::new(),
                                gamepad_id_map: HashMap::new(),
                                symbol_shift,
                                symbol_ctrl,
                                symbol_alt,
                                symbol_logo,
                                symbol_repeat,
                                symbol_left,
                                symbol_right,
                                symbol_middle,
                            }
                        }

                        fn err(&self) -> Option<GameError> {
                            if self.globals.exc_occurred() {
                                Some(GameError::EventLoopError("Script error".to_owned()))
                            } else {
                                None
                            }
                        }

                        fn translate_keycode(&self, keycode: KeyCode) -> Value {
                            self.keycode_list[keycode as usize].clone()
                        }

                        fn translate_button(&self, button: Button) -> Value {
                            self.gamepad_button_map.get(&button).cloned().unwrap()
                        }

                        fn translate_axis(&self, axis: Axis) -> Value {
                            self.gamepad_axis_map.get(&axis).cloned().unwrap()
                        }

                        fn translate_gamepad_id(&mut self, id: GamepadId) -> Value {
                            match self.gamepad_id_map.get(&id) {
                                Some(id) => (*id as i64).into(),
                                None => {
                                    let new_id = self.gamepad_id_list.len();
                                    self.gamepad_id_list.push(id);
                                    self.gamepad_id_map.insert(id, new_id);
                                    (new_id as i64).into()
                                }
                            }
                        }

                        fn translate_modifiers(
                            &self,
                            keymods: KeyMods,
                            repeat: bool,
                        ) -> Vec<Value> {
                            let mut ret = Vec::new();
                            if keymods.contains(KeyMods::SHIFT) {
                                ret.push(self.symbol_shift.into());
                            }
                            if keymods.contains(KeyMods::CTRL) {
                                ret.push(self.symbol_ctrl.into());
                            }
                            if keymods.contains(KeyMods::ALT) {
                                ret.push(self.symbol_alt.into());
                            }
                            if keymods.contains(KeyMods::LOGO) {
                                ret.push(self.symbol_logo.into());
                            }
                            if repeat {
                                ret.push(self.symbol_repeat.into());
                            }
                            ret
                        }
                    }

                    impl<'a> EventHandler for State<'a> {
                        fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
                            if let Some(e) = self.err() {
                                return Err(e);
                            }
                            let update = self.update.borrow().clone();
                            if !update.is_nil() {
                                let context_class = self.context_class;
                                to_game_result(with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(globals, &update, vec![ctx_val])
                                }))?;
                            }
                            Ok(())
                        }

                        fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
                            if let Some(e) = self.err() {
                                return Err(e);
                            }
                            let draw = self.draw.borrow().clone();
                            if !draw.is_nil() {
                                let context_class = self.context_class;
                                to_game_result(with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(globals, &draw, vec![ctx_val])
                                }))?;
                            }
                            graphics::present(ctx)?;
                            if let Some(dur) = self.sleep_per_frame {
                                std::thread::sleep(dur)
                            } else {
                                ggez::timer::yield_now();
                            }
                            Ok(())
                        }

                        fn mouse_button_down_event(
                            &mut self,
                            ctx: &mut Context,
                            button: MouseButton,
                            x: f32,
                            y: f32,
                        ) {
                            if let Some(_) = self.err() {
                                return;
                            }
                            let mouse_down = self.mouse_down.borrow().clone();
                            if !mouse_down.is_nil() {
                                let button: Value = match button {
                                    MouseButton::Left => self.symbol_left.into(),
                                    MouseButton::Right => self.symbol_right.into(),
                                    MouseButton::Middle => self.symbol_middle.into(),
                                    MouseButton::Other(i) => Value::Int(i as i64),
                                };
                                let x = Value::Float(x as f64);
                                let y = Value::Float(y as f64);
                                let context_class = self.context_class;
                                let _r = with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(
                                        globals,
                                        &mouse_down,
                                        vec![ctx_val.clone(), button, x, y],
                                    )
                                });
                            }
                        }

                        fn key_down_event(
                            &mut self,
                            ctx: &mut Context,
                            keycode: KeyCode,
                            keymods: KeyMods,
                            repeat: bool,
                        ) {
                            if let Some(_) = self.err() {
                                return;
                            }
                            let key_down = self.key_down.borrow().clone();
                            if !key_down.is_nil() {
                                let keycode = self.translate_keycode(keycode);
                                let keymods = self.translate_modifiers(keymods, repeat);
                                let context_class = self.context_class;
                                let _r = with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(
                                        globals,
                                        &key_down,
                                        vec![ctx_val.clone(), keycode, keymods.into()],
                                    )
                                });
                            }
                        }

                        fn key_up_event(
                            &mut self,
                            ctx: &mut Context,
                            keycode: KeyCode,
                            keymods: KeyMods,
                        ) {
                            if let Some(_) = self.err() {
                                return;
                            }
                            let key_up = self.key_up.borrow().clone();
                            if !key_up.is_nil() {
                                let keycode = self.translate_keycode(keycode);
                                let keymods = self.translate_modifiers(keymods, false);
                                let context_class = self.context_class;
                                let _r = with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(
                                        globals,
                                        &key_up,
                                        vec![ctx_val.clone(), keycode, keymods.into()],
                                    )
                                });
                            }
                        }

                        fn text_input_event(&mut self, ctx: &mut Context, c: char) {
                            if let Some(_) = self.err() {
                                return;
                            }
                            let text_input = self.text_input.borrow().clone();
                            if !text_input.is_nil() {
                                let cstr = format!("{}", c);
                                let context_class = self.context_class;
                                let _r = with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(
                                        globals,
                                        &text_input,
                                        vec![ctx_val.clone(), cstr.into()],
                                    )
                                });
                            }
                        }

                        fn gamepad_button_down_event(
                            &mut self,
                            ctx: &mut Context,
                            btn: Button,
                            id: GamepadId,
                        ) {
                            if let Some(_) = self.err() {
                                return;
                            }
                            let gamepad_button_down = self.gamepad_button_down.borrow().clone();
                            if !gamepad_button_down.is_nil() {
                                let button = self.translate_button(btn);
                                let id = self.translate_gamepad_id(id);
                                let context_class = self.context_class;
                                let _r = with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(
                                        globals,
                                        &gamepad_button_down,
                                        vec![ctx_val.clone(), button, id],
                                    )
                                });
                            }
                        }

                        fn gamepad_axis_event(
                            &mut self,
                            ctx: &mut Context,
                            axis: Axis,
                            value: f32,
                            id: GamepadId,
                        ) {
                            if let Some(_) = self.err() {
                                return;
                            }
                            let gamepad_axis = self.gamepad_axis.borrow().clone();
                            if !gamepad_axis.is_nil() {
                                let axis = self.translate_axis(axis);
                                let value: Value = (value as f64).into();
                                let id = self.translate_gamepad_id(id);
                                let context_class = self.context_class;
                                let _r = with_ctx(self.globals, ctx, |globals, ctx_val| {
                                    let ctx_val = wrap_ctx(context_class, globals, ctx_val)?;
                                    Eval::call(
                                        globals,
                                        &gamepad_axis,
                                        vec![ctx_val.clone(), axis, value, id],
                                    )
                                });
                            }
                        }
                    }

                    /// wraps the opaque ctx object with the userland provided class
                    fn wrap_ctx(
                        context_class: &Value,
                        globals: &mut Globals,
                        ctx_val: &Value,
                    ) -> EvalResult<Value> {
                        Eval::call(globals, context_class, vec![ctx_val.clone()])
                    }

                    let context_class = &args[0];
                    let name = Eval::expect_string(globals, &args[1])?;
                    let author = Eval::expect_string(globals, &args[2])?;
                    let sleep_per_frame_val = &args[3];
                    let sleep_per_frame = if let Value::Nil = sleep_per_frame_val {
                        None
                    } else {
                        Some(std::time::Duration::from_secs_f64(Eval::expect_floatlike(
                            globals,
                            sleep_per_frame_val,
                        )?))
                    };
                    let (mut ctx, mut event_loop) =
                        ContextBuilder::new(name, author).build().unwrap();
                    let args4 = Eval::expect_cell(globals, &args[4])?.clone();
                    let args5 = Eval::expect_cell(globals, &args[5])?.clone();
                    let args6 = Eval::expect_cell(globals, &args[6])?.clone();
                    let args7 = Eval::expect_cell(globals, &args[7])?.clone();
                    let args8 = Eval::expect_cell(globals, &args[8])?.clone();
                    let args9 = Eval::expect_cell(globals, &args[9])?.clone();
                    let args10 = Eval::expect_cell(globals, &args[10])?.clone();
                    let args11 = Eval::expect_cell(globals, &args[11])?.clone();
                    let mut state = State::new(
                        globals,
                        &mut ctx,
                        context_class,
                        sleep_per_frame,
                        args4,
                        args5,
                        args6,
                        args7,
                        args8,
                        args9,
                        args10,
                        args11,
                    );

                    match event::run(&mut ctx, &mut event_loop, &mut state) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => {
                            if globals.exc_occurred() {
                                Err(ErrorIndicator)
                            } else {
                                globals.set_exc_str(&format!("{:?}", e))
                            }
                        }
                    }
                },
            ),
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

mod wctx {
    // I need to somehow expose the 'Context' variable to the script itself
    // I couldn't figure out a way to do it without any unsafe code, but at least
    // I think I can limit the unsafe to this mod block.
    use super::*;

    pub(super) struct WrappedContext {
        ctx: &'static mut Context,
    }

    impl WrappedContext {
        pub fn get_mut(&mut self) -> &mut Context {
            self.ctx
        }
        pub fn get(&self) -> &Context {
            self.ctx
        }
    }

    pub(super) fn to_ctx<'a>(
        globals: &mut Globals,
        v: &'a Value,
    ) -> EvalResult<Ref<'a, RefCell<WrappedContext>>> {
        Eval::expect_opaque(globals, v)
    }

    pub(super) fn with_ctx<F, R>(globals: &mut Globals, ctx: &mut Context, f: F) -> EvalResult<R>
    where
        F: FnOnce(&mut Globals, &Value) -> EvalResult<R>,
    {
        let wctx = WrappedContext {
            ctx: unsafe { std::mem::transmute::<&'_ mut Context, &'static mut Context>(ctx) },
        };
        let refcell: RefCell<WrappedContext> = RefCell::new(wctx);
        let opaque = Opaque::new(refcell);
        let value: Value = opaque.into();

        let r = f(globals, &value);

        // Make sure to remove the WrappedContext object so that it's no longer
        // accessible from the script (e.g. through any lingering references)
        let _: RefCell<WrappedContext> = Eval::move_opaque(globals, &value).unwrap();

        r
    }
}

// use wctx::WrappedContext;
use wctx::to_ctx;
use wctx::with_ctx;

fn expect_u16(globals: &mut Globals, v: &Value) -> EvalResult<u16> {
    let x = Eval::expect_int(globals, v)?;
    Eval::check_u16(globals, x)
}

fn expect_u32(globals: &mut Globals, v: &Value) -> EvalResult<u32> {
    let x = Eval::expect_int(globals, v)?;
    Eval::check_u32(globals, x)
}

fn expect_point(globals: &mut Globals, point: &Value) -> EvalResult<Point> {
    let (x, y) = Eval::unpack_pair(globals, point)?;
    let x = Eval::expect_floatlike(globals, &x)?;
    let y = Eval::expect_floatlike(globals, &y)?;
    Ok(mkpt(x as f32, y as f32))
}

fn expect_f32_pair(globals: &mut Globals, value: &Value) -> EvalResult<(f32, f32)> {
    let (a, b) = Eval::unpack_pair(globals, value)?;
    let a = Eval::expect_floatlike(globals, &a)? as f32;
    let b = Eval::expect_floatlike(globals, &b)? as f32;
    Ok((a, b))
}

fn expect_4_f32(globals: &mut Globals, value: &Value) -> EvalResult<(f32, f32, f32, f32)> {
    let (a, b, c, d) = Eval::unpack4(globals, value)?;
    let a = Eval::expect_floatlike(globals, &a)? as f32;
    let b = Eval::expect_floatlike(globals, &b)? as f32;
    let c = Eval::expect_floatlike(globals, &c)? as f32;
    let d = Eval::expect_floatlike(globals, &d)? as f32;
    Ok((a, b, c, d))
}

fn from_color(_globals: &mut Globals, color: Color) -> EvalResult<Value> {
    let opaque = Opaque::new(color);
    Ok(opaque.into())
}

fn to_color_ref<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<Ref<'a, Color>> {
    Eval::expect_opaque(globals, value)
}

fn to_game_result<R>(eval_result: EvalResult<R>) -> GameResult<R> {
    match eval_result {
        Ok(r) => Ok(r),
        Err(_) => Err(GameError::EventLoopError("script error".to_owned())),
    }
}

fn from_mesh_builder(_globals: &mut Globals, mesh_builder: MeshBuilder) -> EvalResult<Value> {
    let mesh_builder: RefCell<MeshBuilder> = RefCell::new(mesh_builder);
    let opaque = Opaque::new(mesh_builder);
    Ok(opaque.into())
}

fn to_mesh_builder_ref<'a>(
    globals: &mut Globals,
    value: &'a Value,
) -> EvalResult<Ref<'a, RefCell<MeshBuilder>>> {
    Eval::expect_opaque(globals, value)
}

fn to_mesh_builder(globals: &mut Globals, value: &Value) -> EvalResult<MeshBuilder> {
    let refcell: RefCell<MeshBuilder> = Eval::move_opaque(globals, value)?;
    Ok(refcell.into_inner())
}

fn from_mesh(_globals: &mut Globals, mesh: Mesh) -> EvalResult<Value> {
    let drawable: EDrawable = EDrawable::Mesh(mesh);
    let opaque = Opaque::new(drawable);
    Ok(opaque.into())
}

fn from_text(_globals: &mut Globals, text: Text) -> EvalResult<Value> {
    let drawable: EDrawable = EDrawable::Text(text);
    let opaque = Opaque::new(drawable);
    Ok(opaque.into())
}

fn from_spriteidx(_globals: &mut Globals, sid: SpriteIdx) -> EvalResult<Value> {
    Ok(Opaque::new(sid).into())
}

fn copy_spriteidx(globals: &mut Globals, value: &Value) -> EvalResult<SpriteIdx> {
    let sid: Ref<SpriteIdx> = Eval::expect_opaque(globals, value)?;
    Ok(*sid)
}

fn from_image(_globals: &mut Globals, image: Image) -> EvalResult<Value> {
    let drawable: EDrawable = EDrawable::Image(image);
    let opaque = Opaque::new(drawable);
    Ok(opaque.into())
}

fn from_sprite_batch(_globals: &mut Globals, sprite_batch: SpriteBatch) -> EvalResult<Value> {
    let drawable: EDrawable = EDrawable::SpriteBatch(sprite_batch);
    let opaque = Opaque::new(drawable);
    Ok(opaque.into())
}

fn to_drawable<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<Ref<'a, EDrawable>> {
    Eval::expect_opaque(globals, value)
}

fn to_text<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<Ref<'a, Text>> {
    let drawable = to_drawable(globals, value)?;
    if drawable.text().is_some() {
        Ok(Ref::map(to_drawable(globals, value)?, |drawable| {
            drawable.text().unwrap()
        }))
    } else {
        globals.set_exc_str(&format!(
            "Expected Drawable Text, but got a different drawable"
        ))
    }
}

fn to_image<'a>(globals: &mut Globals, value: &'a Value) -> EvalResult<Ref<'a, Image>> {
    let drawable = to_drawable(globals, value)?;
    if drawable.image().is_some() {
        Ok(Ref::map(to_drawable(globals, value)?, |drawable| {
            drawable.image().unwrap()
        }))
    } else {
        globals.set_exc_str(&format!(
            "Expected Drawable Image, but got a different drawable"
        ))
    }
}

fn to_sprite_batch_mut<'a>(
    globals: &mut Globals,
    value: &'a Value,
) -> EvalResult<RefMut<'a, SpriteBatch>> {
    let mut drawable: RefMut<EDrawable> = Eval::expect_opaque_mut(globals, value)?;
    if drawable.sprite_batch_mut().is_some() {
        Ok(RefMut::map(drawable, |drawable| {
            drawable.sprite_batch_mut().unwrap()
        }))
    } else {
        globals.set_exc_str(&format!(
            "Expected Drawable SpriteBatch, but got a different drawable"
        ))
    }
}

enum EDrawable {
    Mesh(Mesh),
    Text(Text),
    Image(Image),
    SpriteBatch(SpriteBatch),
}

impl EDrawable {
    fn text(&self) -> Option<&Text> {
        if let EDrawable::Text(text) = self {
            Some(text)
        } else {
            None
        }
    }
    // fn is_mesh(&self) -> bool {
    //     if let EDrawable::Mesh(_) = self { true } else { false }
    // }
    fn image(&self) -> Option<&Image> {
        if let EDrawable::Image(image) = self {
            Some(image)
        } else {
            None
        }
    }
    fn sprite_batch_mut(&mut self) -> Option<&mut SpriteBatch> {
        if let EDrawable::SpriteBatch(sprite_batch) = self {
            Some(sprite_batch)
        } else {
            None
        }
    }
}

fn draw(ctx: &mut Context, drawable: &EDrawable, params: graphics::DrawParam) -> GameResult<()> {
    match drawable {
        EDrawable::Mesh(mesh) => graphics::draw(ctx, mesh, params),
        EDrawable::Text(text) => graphics::draw(ctx, text, params),
        EDrawable::Image(image) => graphics::draw(ctx, image, params),
        EDrawable::SpriteBatch(sprite_batch) => graphics::draw(ctx, sprite_batch, params),
    }
}

fn list_gamepad_buttons(globals: &mut Globals) -> HashMap<Button, Value> {
    vec![
        (Button::South, globals.intern_str("South").into()),
        (Button::East, globals.intern_str("East").into()),
        (Button::North, globals.intern_str("North").into()),
        (Button::West, globals.intern_str("West").into()),
        (Button::C, globals.intern_str("C").into()),
        (Button::Z, globals.intern_str("Z").into()),
        (
            Button::LeftTrigger,
            globals.intern_str("LeftTrigger").into(),
        ),
        (
            Button::LeftTrigger2,
            globals.intern_str("LeftTrigger2").into(),
        ),
        (
            Button::RightTrigger,
            globals.intern_str("RightTrigger").into(),
        ),
        (
            Button::RightTrigger2,
            globals.intern_str("RightTrigger2").into(),
        ),
        (Button::Select, globals.intern_str("Select").into()),
        (Button::Start, globals.intern_str("Start").into()),
        (Button::Mode, globals.intern_str("Mode").into()),
        (Button::LeftThumb, globals.intern_str("LeftThumb").into()),
        (Button::RightThumb, globals.intern_str("RightThumb").into()),
        (Button::DPadUp, globals.intern_str("DPadUp").into()),
        (Button::DPadDown, globals.intern_str("DPadDown").into()),
        (Button::DPadLeft, globals.intern_str("DPadLeft").into()),
        (Button::DPadRight, globals.intern_str("DPadRight").into()),
        (Button::Unknown, globals.intern_str("Unknown").into()),
    ]
    .into_iter()
    .collect()
}

fn list_gamepad_axes(globals: &mut Globals) -> HashMap<Axis, Value> {
    vec![
        (Axis::LeftStickX, globals.intern_str("LeftStickX").into()),
        (Axis::LeftStickY, globals.intern_str("LeftStickY").into()),
        (Axis::LeftZ, globals.intern_str("LeftZ").into()),
        (Axis::RightStickX, globals.intern_str("RightStickX").into()),
        (Axis::RightStickY, globals.intern_str("RightStickY").into()),
        (Axis::RightZ, globals.intern_str("RightZ").into()),
        (Axis::DPadX, globals.intern_str("DPadX").into()),
        (Axis::DPadY, globals.intern_str("DPadY").into()),
        (Axis::Unknown, globals.intern_str("Unknown").into()),
    ]
    .into_iter()
    .collect()
}

fn list_keycodes() -> Vec<KeyCode> {
    // TOOD: Figure out how to do this without having to copy and
    // paste the entire enum
    let keycodes = vec![
        KeyCode::Key1,
        KeyCode::Key2,
        KeyCode::Key3,
        KeyCode::Key4,
        KeyCode::Key5,
        KeyCode::Key6,
        KeyCode::Key7,
        KeyCode::Key8,
        KeyCode::Key9,
        KeyCode::Key0,
        KeyCode::A,
        KeyCode::B,
        KeyCode::C,
        KeyCode::D,
        KeyCode::E,
        KeyCode::F,
        KeyCode::G,
        KeyCode::H,
        KeyCode::I,
        KeyCode::J,
        KeyCode::K,
        KeyCode::L,
        KeyCode::M,
        KeyCode::N,
        KeyCode::O,
        KeyCode::P,
        KeyCode::Q,
        KeyCode::R,
        KeyCode::S,
        KeyCode::T,
        KeyCode::U,
        KeyCode::V,
        KeyCode::W,
        KeyCode::X,
        KeyCode::Y,
        KeyCode::Z,
        KeyCode::Escape,
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
        KeyCode::F9,
        KeyCode::F10,
        KeyCode::F11,
        KeyCode::F12,
        KeyCode::F13,
        KeyCode::F14,
        KeyCode::F15,
        KeyCode::F16,
        KeyCode::F17,
        KeyCode::F18,
        KeyCode::F19,
        KeyCode::F20,
        KeyCode::F21,
        KeyCode::F22,
        KeyCode::F23,
        KeyCode::F24,
        KeyCode::Snapshot,
        KeyCode::Scroll,
        KeyCode::Pause,
        KeyCode::Insert,
        KeyCode::Home,
        KeyCode::Delete,
        KeyCode::End,
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::Left,
        KeyCode::Up,
        KeyCode::Right,
        KeyCode::Down,
        KeyCode::Back,
        KeyCode::Return,
        KeyCode::Space,
        KeyCode::Compose,
        KeyCode::Caret,
        KeyCode::Numlock,
        KeyCode::Numpad0,
        KeyCode::Numpad1,
        KeyCode::Numpad2,
        KeyCode::Numpad3,
        KeyCode::Numpad4,
        KeyCode::Numpad5,
        KeyCode::Numpad6,
        KeyCode::Numpad7,
        KeyCode::Numpad8,
        KeyCode::Numpad9,
        KeyCode::AbntC1,
        KeyCode::AbntC2,
        KeyCode::Add,
        KeyCode::Apostrophe,
        KeyCode::Apps,
        KeyCode::At,
        KeyCode::Ax,
        KeyCode::Backslash,
        KeyCode::Calculator,
        KeyCode::Capital,
        KeyCode::Colon,
        KeyCode::Comma,
        KeyCode::Convert,
        KeyCode::Decimal,
        KeyCode::Divide,
        KeyCode::Equals,
        KeyCode::Grave,
        KeyCode::Kana,
        KeyCode::Kanji,
        KeyCode::LAlt,
        KeyCode::LBracket,
        KeyCode::LControl,
        KeyCode::LShift,
        KeyCode::LWin,
        KeyCode::Mail,
        KeyCode::MediaSelect,
        KeyCode::MediaStop,
        KeyCode::Minus,
        KeyCode::Multiply,
        KeyCode::Mute,
        KeyCode::MyComputer,
        KeyCode::NavigateForward,  // also called "Prior"
        KeyCode::NavigateBackward, // also called "Next"
        KeyCode::NextTrack,
        KeyCode::NoConvert,
        KeyCode::NumpadComma,
        KeyCode::NumpadEnter,
        KeyCode::NumpadEquals,
        KeyCode::OEM102,
        KeyCode::Period,
        KeyCode::PlayPause,
        KeyCode::Power,
        KeyCode::PrevTrack,
        KeyCode::RAlt,
        KeyCode::RBracket,
        KeyCode::RControl,
        KeyCode::RShift,
        KeyCode::RWin,
        KeyCode::Semicolon,
        KeyCode::Slash,
        KeyCode::Sleep,
        KeyCode::Stop,
        KeyCode::Subtract,
        KeyCode::Sysrq,
        KeyCode::Tab,
        KeyCode::Underline,
        KeyCode::Unlabeled,
        KeyCode::VolumeDown,
        KeyCode::VolumeUp,
        KeyCode::Wake,
        KeyCode::WebBack,
        KeyCode::WebFavorites,
        KeyCode::WebForward,
        KeyCode::WebHome,
        KeyCode::WebRefresh,
        KeyCode::WebSearch,
        KeyCode::WebStop,
        KeyCode::Yen,
        KeyCode::Copy,
        KeyCode::Paste,
        KeyCode::Cut,
    ];
    for (i, keycode) in keycodes.iter().enumerate() {
        assert_eq!(i, *keycode as usize);
    }
    keycodes
}

fn list_keycode_symbols(globals: &mut Globals) -> Vec<Value> {
    let mut ret = Vec::new();
    for keycode in list_keycodes() {
        ret.push(globals.intern_str(&format!("{:?}", keycode)).into());
    }
    ret.into()
}
