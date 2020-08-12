use super::*;
use ggez::graphics::Drawable;

mod conv;
mod geo;
mod mesh;
pub use conv::*;
pub use geo::*;
pub use mesh::*;

pub const NAME: &str = "a.ggez.graphics";

pub(in super::super) fn new() -> NativeModule {
    NativeModule::new(NAME, |m| {
        m.func("width", [], "", |globals, _, _| {
            let ctx = getctx(globals)?;
            Ok(ggez::graphics::drawable_size(ctx).0.into())
        });
        m.func("height", [], "", |globals, _, _| {
            let ctx = getctx(globals)?;
            Ok(ggez::graphics::drawable_size(ctx).1.into())
        });
        m.func("clear", ["color"], "", |globals, args, _| {
            let mut args = args.into_iter();
            let color = Color::try_from(args.next().unwrap())?;
            let ctx = getctx(globals)?;
            ggez::graphics::clear(ctx, color.into());
            Ok(Value::Nil)
        });
        m.func(
            "draw",
            ArgSpec::builder()
                .req("drawable")
                .def("x", 0)
                .def("y", 0)
                .def("rotation", 0)
                .def("xscale", 1)
                .def("yscale", 1)
                .def("xoffset", 0)
                .def("yoffset", 0)
                .def("color", ()),
            "",
            |globals, args, _| {
                let mut args = args.into_iter();
                let drawparam = DrawParam::default();
                let drawable = args.next().unwrap();
                let x = f32::try_from(args.next().unwrap())?;
                let y = f32::try_from(args.next().unwrap())?;
                let rotation = f32::try_from(args.next().unwrap())?;
                let xscale = f32::try_from(args.next().unwrap())?;
                let yscale = f32::try_from(args.next().unwrap())?;
                let xoffset = f32::try_from(args.next().unwrap())?;
                let yoffset = f32::try_from(args.next().unwrap())?;
                drawparam
                    .dest([x, y])
                    .rotation(rotation)
                    .scale([xscale, yscale])
                    .offset([xoffset, yoffset]);
                let color = args.next().unwrap();
                if !color.is_nil() {
                    let color = Color::try_from(args.next().unwrap())?;
                    drawparam.color(color.into());
                }

                let ctx = getctx(globals)?;
                if drawable.is_handle::<Text>() {
                    mtry!(ggez::graphics::draw(
                        ctx,
                        drawable.to_xref::<Text>(globals)?.get(),
                        drawparam,
                    ));
                } else if drawable.is_handle::<Mesh>() {
                    mtry!(ggez::graphics::draw(
                        ctx,
                        drawable.to_xref::<Mesh>(globals)?.get(),
                        drawparam,
                    ));
                } else {
                    return Err(rterr!("Expected drawable but got {:?}", drawable));
                }

                Ok(Value::Nil)
            },
        );
        m.func(
            "print",
            ArgSpec::builder().req("text").def("x", 0).def("y", 0),
            "",
            |globals, args, _| {
                let mut args = args.into_iter();
                let textval = args.next().unwrap();
                let text = textval.to_xref::<Text>(globals)?;
                let x = f32::try_from(args.next().unwrap())?;
                let y = f32::try_from(args.next().unwrap())?;
                let ctx = getctx(globals)?;
                mtry!(ggez::graphics::draw(
                    ctx,
                    text.get(),
                    DrawParam::default().dest([x, y])
                ));
                Ok(Value::Nil)
            },
        );
        m.func(
            "queue_text",
            ArgSpec::builder()
                .req("text")
                .def("x", 0)
                .def("y", 0)
                .def("color", ()),
            "",
            |globals, args, _| {
                let mut args = args.into_iter();
                let textval = args.next().unwrap();
                let text = textval.to_xref::<Text>(globals)?;
                let x = f32::try_from(args.next().unwrap())?;
                let y = f32::try_from(args.next().unwrap())?;
                let color: Option<ggez::graphics::Color> = match args.next().unwrap() {
                    Value::Nil => None,
                    value => Some(Color::try_from(value)?.into()),
                };
                let ctx = getctx(globals)?;
                ggez::graphics::queue_text(ctx, text.get(), [x, y], color);
                Ok(Value::Nil)
            },
        );
        m.func(
            "draw_queued_text",
            ArgSpec::builder().def("x", 0).def("y", 0),
            "",
            |globals, args, _| {
                let mut args = args.into_iter();
                let x = f32::try_from(args.next().unwrap())?;
                let y = f32::try_from(args.next().unwrap())?;
                let ctx = getctx(globals)?;
                mtry!(ggez::graphics::draw_queued_text(
                    ctx,
                    DrawParam::default().dest([x, y]),
                    None,
                    ggez::graphics::FilterMode::Linear,
                ));
                Ok(Value::Nil)
            },
        );
        m.func("set_window_title", ["title"], "", |globals, args, _| {
            let mut args = args.into_iter();
            let title = args.next().unwrap().into_string()?;
            let ctx = getctx(globals)?;
            ggez::graphics::set_window_title(ctx, title.str());
            Ok(Value::Nil)
        });
        m.class::<Text, _>("Text", |cls| {
            cls.sfunc("__call", ["arg"], "", |globals, args, _| {
                let mut args = args.into_iter();
                let arg = args.next().unwrap();
                let text = arg.convert::<Text>(globals)?;
                Ok(globals.new_handle(text)?.into())
            });
            cls.ifunc("width", [], "", |owner, globals, _, _| {
                let ctx = getctx(globals)?;
                Ok(owner.borrow().get().width(ctx).into())
            });
            cls.ifunc("height", [], "", |owner, globals, _, _| {
                let ctx = getctx(globals)?;
                Ok(owner.borrow().get().height(ctx).into())
            });
            cls.ifunc("contents", [], "", |owner, _globals, _, _| {
                Ok(owner.borrow().get().contents().into())
            });
        });
        m.class::<MeshBuilder, _>("MeshBuilder", |cls| {
            cls.sfunc("__call", [], "", |globals, _, _| {
                Ok(globals.new_handle(MeshBuilder::new())?.into())
            });
            cls.ifunc("build", [], "", |owner, globals, _, _| {
                let ctx = getctx(globals)?;
                let mesh = owner.borrow_mut().build(ctx)?;
                Ok(globals.new_handle(mesh)?.into())
            });
            cls.ifunc(
                "circle",
                ["mode", "point", "radius", "tolerance", "color"],
                "",
                |owner, _globals, args, _| {
                    let mut args = args.into_iter();
                    let mode = DrawMode::try_from(args.next().unwrap())?;
                    let [x, y] = <[f32; 2]>::try_from(args.next().unwrap())?;
                    let radius = args.next().unwrap().f32()?;
                    let tolerance = args.next().unwrap().f32()?;
                    let color = Color::try_from(args.next().unwrap())?;
                    owner
                        .borrow_mut()
                        .get_mut()?
                        .circle(mode.into(), [x, y], radius, tolerance, color.into());
                    Ok(owner.into())
                },
            );
        });
        m.class::<Mesh, _>("Mesh", |cls| {
            cls.ifunc("width", [], "", |owner, globals, _, _| {
                let ctx = getctx(globals)?;
                match owner.borrow().get().dimensions(ctx) {
                    Some(rect) => Ok(rect.w.into()),
                    None => Ok(Value::Nil),
                }
            });
            cls.ifunc("height", [], "", |owner, globals, _, _| {
                let ctx = getctx(globals)?;
                match owner.borrow().get().dimensions(ctx) {
                    Some(rect) => Ok(rect.h.into()),
                    None => Ok(Value::Nil),
                }
            });
        });
    })
}
