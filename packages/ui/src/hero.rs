use dioxus::prelude::*;

const HERO_CSS: Asset = asset!("/assets/styling/hero.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");

#[component]
pub fn Hero() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: HERO_CSS }

        div {
            id: "hero",
            img { src: HEADER_SVG, id: "header" }
            h1 { "Samantha" }
            div { id: "links",
                a { href: "https://dioxuslabs.com/learn/0.7/", "📚 Learn Dioxus" }
                a { href: "https://github.com/RustPython/RustPython", "RustPython"}
            }
        }
    }
}
