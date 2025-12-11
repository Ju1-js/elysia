use freya::{core::custom_attributes::NodeReferenceLayout, prelude::*};

use crate::components::Expand;

#[derive(Props, Clone, PartialEq)]
pub struct MyAnimatedCarouselProps {
    pub items: Vec<Element>,
    #[props(optional)]
    pub selected: Option<Signal<usize>>,
}

#[component]
pub fn MyAnimatedCarousel(props: MyAnimatedCarouselProps) -> Element {
    let MyAnimatedCarouselProps { items, selected } = props;
    let (reference, node_size) = use_node_signal();
    let mut state = use_signal(|| CarouselState::Stopped(0));

    let len = items.len();
    let onwheel = move |e: Event<WheelData>| {
        let direction = e.get_delta_y().signum();

        let current: CarouselState = *state.read();

        match current {
            CarouselState::Stopped(index) => {
                if direction > 0.0 && index < len - 1 {
                    *state.write() = CarouselState::Running(index, index + 1);
                    if let Some(mut sel) = selected {
                        *sel.write() = index + 1;
                    }
                } else if direction < 0.0 && index > 0 {
                    *state.write() = CarouselState::Running(index, index - 1);
                    if let Some(mut sel) = selected {
                        *sel.write() = index - 1;
                    }
                }
            }
            CarouselState::Running(_, _) => {}
        };
    };

    rsx!(
        rect {
            onwheel,
            reference,
            Carousel { items, state, node_size }
        }
    )
}

#[component]
fn Carousel(
    items: Vec<Element>,
    state: Signal<CarouselState>,
    node_size: ReadOnlySignal<NodeReferenceLayout>,
) -> Element {
    let animation = use_animation(move |_conf| {
        AnimNum::new(1.0, 0.0)
            .time(250)
            .ease(Ease::Out)
            .function(Function::Cubic)
    });

    use_effect(move || {
        let current: CarouselState = *state.read();

        match current {
            CarouselState::Running(_, to) => {
                if !animation.is_running() {
                    if animation.has_run_yet() {
                        *state.write() = CarouselState::Stopped(to);
                    } else {
                        animation.run(AnimDirection::Forward);
                    }
                }
            }
            CarouselState::Stopped(_) => {
                animation.reset();
            }
        };
    });

    let opacity = animation.get().read().read();

    rsx!(
        rect {
            overflow: "clip",
            width: "100%",
            corner_radius: "16",
            direction: "horizontal",

            {
                if items.is_empty() {
                    rsx!(
                        rect {
                            width: "100%",
                            height: "200",
                            main_align: "center",
                            cross_align: "center",
                            label {
                                "No content available"
                            }
                        }
                    )
                } else {
                    match *state.read() {
                        CarouselState::Stopped(index) => {
                            let index = if index >= items.len() {0} else {index};
                            rsx! {
                                rect {
                                    width: "100%",
                                    {&items[index]}
                                }
                            }
                        }
                        CarouselState::Running(from, to) => {
                            let from = if from >= items.len() {0} else {from};
                            let to = if to >= items.len() {0} else {to};

                            rsx! {
                                rect {
                                    width: "100%",
                                    position: "relative",
                                    
                                    // Fade in
                                    rect {
                                        width: "100%",
                                        opacity: "{1.0 - opacity}",
                                        {&items[to]}
                                    }
                                    
                                    // Fade out
                                    rect {
                                        width: "100%",
                                        position: "absolute",
                                        position_top: "0",
                                        position_left: "0",
                                        opacity: "{opacity}",
                                        {&items[from]}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CarouselState {
    Stopped(usize),
    Running(usize, usize),
}
