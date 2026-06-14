use yew::prelude::*;
use yew_router::prelude::*;

use crate::app::Route;
use crate::components::landing::{
    LandingFeatures, LandingFooter, LandingHero, LandingHowItWorks, LandingPreview, LandingStats,
    LandingTransparency,
};
use crate::services::auth::is_authenticated;

#[component]
pub fn Landing() -> Html {
    let navigator = use_navigator();

    use_effect_with((), move |()| {
        if is_authenticated() {
            if let Some(nav) = navigator {
                nav.push(&Route::Dashboard);
            }
        }
    });

    if is_authenticated() {
        return html! {};
    }

    html! {
        <div class="gi-l-page">
            <LandingHero />
            <LandingStats />
            <LandingPreview />
            <LandingFeatures />
            <LandingHowItWorks />
            <LandingTransparency />
            <LandingFooter />
        </div>
    }
}
