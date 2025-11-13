pub struct Asset {
    pub body: &'static [u8],
    pub content_type: &'static str,
}

macro_rules! asset {
    ($path:literal, $mime:literal) => {
        Asset {
            body: include_bytes!($path),
            content_type: $mime,
        }
    };
}

pub fn get(path: &str) -> Option<Asset> {
    match path {
        "styles.css" => Some(asset!("static/styles.css", "text/css; charset=utf-8")),
        "app.js" => Some(asset!(
            "static/app.js",
            "application/javascript; charset=utf-8"
        )),
        "lib/deps.js" => Some(asset!(
            "static/lib/deps.js",
            "application/javascript; charset=utf-8"
        )),
        "lib/flowGraph.js" => Some(asset!(
            "static/lib/flowGraph.js",
            "application/javascript; charset=utf-8"
        )),
        "hooks/useDashboardData.js" => Some(asset!(
            "static/hooks/useDashboardData.js",
            "application/javascript; charset=utf-8"
        )),
        "components/DashboardApp.js" => Some(asset!(
            "static/components/DashboardApp.js",
            "application/javascript; charset=utf-8"
        )),
        "components/Header.js" => Some(asset!(
            "static/components/Header.js",
            "application/javascript; charset=utf-8"
        )),
        "components/InsightsPanel.js" => Some(asset!(
            "static/components/InsightsPanel.js",
            "application/javascript; charset=utf-8"
        )),
        "components/AgentRoster.js" => Some(asset!(
            "static/components/AgentRoster.js",
            "application/javascript; charset=utf-8"
        )),
        "components/EventStream.js" => Some(asset!(
            "static/components/EventStream.js",
            "application/javascript; charset=utf-8"
        )),
        "components/FlowPanel.js" => Some(asset!(
            "static/components/FlowPanel.js",
            "application/javascript; charset=utf-8"
        )),
        "components/VisualPanel.js" => Some(asset!(
            "static/components/VisualPanel.js",
            "application/javascript; charset=utf-8"
        )),
        _ => None,
    }
}
