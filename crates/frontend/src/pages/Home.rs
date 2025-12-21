use freya::prelude::*;

#[allow(non_snake_case)]
#[component]
pub fn Home() -> Element {
    rsx! {
        rect {
            width: "fill",
            height: "fill",
            cross_align: "center",
            main_align: "center",
            direction: "vertical",
            rect {
                position: "absolute",
                position_top: "0",
                position_left: "0",
                width: "100%",
                height: "100%",
                layer: "-1",
                background: "linear-gradient(135deg, rgb(30,30,40) 0%, rgb(15,15,20) 100%)",
            },
            rect {
                direction: "vertical",
                cross_align: "center",
                main_align: "center",
                spacing: "24",
                padding: "40",
                background: "rgb(35,35,45,0.5)",
                corner_radius: "20",
                shadow: "0 8 32 0 rgb(0, 0, 0, 60)",
                backdrop_blur: "16",
                max_width: "600",
                image {
                    width: "120",
                    height: "120",
                    sampling: "trilinear",
                    image_data: static_bytes(include_bytes!("../../../../assets/elysia.png"))
                },
                label {
                    font_family: "Noto Sans",
                    font_size: "32",
                    font_weight: "700",
                    "Welcome to Elysia!"
                },
                label {
                    font_family: "Noto Sans",
                    font_size: "16",
                    color: "rgb(200,200,210)",
                    text_align: "center",
                    "Select a game from the sidebar to get started."
                },
            },
        }
    }
}
