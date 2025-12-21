use freya::prelude::*;

#[component]
pub fn Expand(children: Element) -> Element {
    rsx!(
        rect {
            height: "100%",
            width: "100%",
            main_align: "center",
            cross_align: "center",
            {children}
        }
    )
}
