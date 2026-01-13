use freya::prelude::*;

#[allow(non_snake_case)]
/// Error page component
#[component]
pub fn ErrorPage() -> Element {
    rsx!(
        label {
            "Error"
        }
    )
}
