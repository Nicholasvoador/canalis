//! HOMEPAGE: KANALI's live dashboard, plus device and computer specifications.

use super::page_scroll;
use crate::ui::assets;
use crate::ui::charts::{self, GradientFill, Sparkline};
use crate::ui::geometry::Area;
use crate::ui::shell::CanalisWindow;
use crate::ui::theme::{CARD_RAISED, DANGER, DOWNLOAD_GREEN, LIME, MUTED, OK, ORANGE, TEXT, UPLOAD_BLUE};
use crate::ui::widgets::{CARD_GAP, bold, card, content_width, divider, info_line, muted, page_header, status_dot, tabs};
use crate::status::LiveStatus;
use crate::ui::format;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

const MIN_COLUMN_WIDTH: f32 = 230.0;
const MIN_ROW_HEIGHT: f32 = 230.0;
const MAX_ROW_HEIGHT: f32 = 340.0;
const SPEC_CARD_MIN_WIDTH: f32 = 360.0;
/// The page's top and bottom margins, its header, the tabs and the gaps between them.
const HEIGHT_ABOVE_DASHBOARD: f32 = 194.0;
const MEMORY_GRADIENT: [u32; 2] = [CARD_RAISED, LIME];
const DISK_GRADIENT: [u32; 3] = [CARD_RAISED, LIME, ORANGE];

#[derive(Clone, Copy, PartialEq, Default)]
pub enum HomeTab {
    #[default]
    Dashboard,
    Specifications,
}

impl CanalisWindow {
    pub fn home_page(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let tabs = tabs(self.home_tab, &[(HomeTab::Dashboard, "Dashboard"), (HomeTab::Specifications, "Specifications")], |this, tab| this.home_tab = tab, cx);
        let content = match self.home_tab {
            HomeTab::Dashboard => self.dashboard(window).into_any_element(),
            HomeTab::Specifications => self.specifications(window).into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(page_header("HOMEPAGE"))
            .child(div().flex().justify_between().items_start().child(tabs).child(self.connection_status()))
            .child(divider())
            .child(div().h(px(CARD_GAP)).flex_none())
            .child(page_scroll("home-scroll", content))
            .into_any_element()
    }

    fn connection_status(&self) -> impl IntoElement {
        if self.connected() {
            status_dot(OK, "Cooler screen connected")
        } else if self.service_running() {
            status_dot(LIME, "Looking for the cooler screen…")
        } else {
            status_dot(DANGER, "Service not running — start it in SETTING")
        }
    }

    fn dashboard(&mut self, window: &mut Window) -> impl IntoElement {
        // The network graphs scroll smoothly, so the dashboard draws every frame while it is open.
        window.request_animation_frame();
        let viewport = window.viewport_size();
        let width = content_width(window);
        let row_height = ((f32::from(viewport.height) - HEIGHT_ABOVE_DASHBOARD - CARD_GAP) / 2.0).clamp(MIN_ROW_HEIGHT, MAX_ROW_HEIGHT);
        let layout = DashboardLayout::fit(width, row_height);
        let unit = self.editor.settings.temperature_unit.clone();
        let status = self.status.current.clone();
        let cards = [
            (layout.network, self.network_card(layout.network).into_any_element()),
            (layout.cpu, self.gauge_card(&LoadGauge::read(&status, Processor::Cpu, &unit)).into_any_element()),
            (layout.gpu, self.gauge_card(&LoadGauge::read(&status, Processor::Gpu, &unit)).into_any_element()),
            (layout.memory, self.bar_card(&memory_bar(&status)).into_any_element()),
            (layout.disk, self.bar_card(&disk_bar(&status)).into_any_element()),
        ];
        let size = layout.size();
        div().relative().w(px(size.width)).h(px(size.height)).flex_none().children(cards.into_iter().map(|(area, card)| placed(area, card)))
    }

    fn network_card(&mut self, area: Area) -> impl IntoElement {
        let download = NetworkSeries { name: "Download", samples: self.network.download().to_vec(), color: DOWNLOAD_GREEN, icon: assets::ARROW_DOWN };
        let upload = NetworkSeries { name: "Upload", samples: self.network.upload().to_vec(), color: UPLOAD_BLUE, icon: assets::ARROW_UP };
        let graphs = if graphs_side_by_side(area.width, area.height) { div().flex() } else { div().flex().flex_col() };
        let graphs = graphs.flex_1().min_h_0().gap(px(CARD_GAP)).child(self.network_graph(download)).child(self.network_graph(upload));
        card().size_full().child(bold("Network Status", 19.0)).child(graphs)
    }

    fn network_graph(&mut self, series: NetworkSeries) -> impl IntoElement {
        let peak = self.easing.value(series.peak_key(), charts::sparkline_peak(&series.samples) as f64) as f32;
        let rate = self.easing.value(series.rate_key(), series.samples.last().copied().unwrap_or(0.0) as f64);
        let scroll = self.network.scroll();
        let color = series.color;
        let samples = series.samples;
        let graph = canvas(|_, _, _| {}, move |bounds, (), window, _| charts::sparkline(window, Area::from_bounds(bounds), &Sparkline { samples: &samples, peak, scroll, color }));
        let (value, unit) = format::rate(rate);
        let reading = div()
            .flex()
            .items_center()
            .gap_2()
            .h(px(26.0))
            .child(svg().path(series.icon).size(px(20.0)).text_color(rgb(color)))
            .child(format!("{}:", series.name))
            .child(div().font_weight(FontWeight::BOLD).child(value))
            .child(div().text_size(px(11.5)).text_color(rgb(MUTED)).child(unit));
        div().flex().flex_col().flex_1().min_h_0().gap_2().pt_3().child(graph.flex_1().w_full()).child(reading)
    }

    fn gauge_card(&mut self, gauge: &LoadGauge) -> impl IntoElement {
        let load = self.easing.value(gauge.label, gauge.load);
        let fraction = (load / 100.0) as f32;
        let dial = canvas(|_, _, _| {}, move |bounds, (), window, _| charts::gauge(window, Area::from_bounds(bounds), fraction));
        card_with_heading(load, gauge.label).child(dial.flex_1().w_full()).child(footer(&gauge.footer))
    }

    fn bar_card(&mut self, bar: &LoadBar) -> impl IntoElement {
        let load = self.easing.value(bar.label, bar.load);
        let fill = GradientFill { fraction: (load / 100.0) as f32, stops: bar.stops };
        let track = canvas(|_, _, _| {}, move |bounds, (), window, _| charts::gradient_bar(window, Area::from_bounds(bounds), &fill));
        let middle = div().flex_1().flex().items_center().child(track.h(px(26.0)).w_full());
        card_with_heading(load, bar.label).child(middle).child(footer(&bar.footer))
    }

    fn specifications(&self, window: &Window) -> impl IntoElement {
        let width = content_width(window);
        let side_by_side = width >= 2.0 * SPEC_CARD_MIN_WIDTH + CARD_GAP;
        let cards = if side_by_side { div().flex() } else { div().flex().flex_col() };
        cards.gap(px(CARD_GAP)).child(self.panorama_card().flex_1()).child(computer_card(self.status()).flex_1())
    }

    fn panorama_card(&self) -> Div {
        card().child(bold("Panorama", 19.0)).child(device_identity(self.status())).child(divider()).child(self.device_readings())
    }

    fn device_readings(&self) -> impl IntoElement {
        let (status, connected) = (self.status(), self.connected());
        let error = status.text("/error");
        div()
            .flex()
            .flex_col()
            .child(info_line("Pump-head fan", rpm(status.number("/device_status/status/fanLCD"), connected)))
            .child(info_line("Pump", rpm(status.number("/device_status/status/turboPump"), connected)))
            .child(info_line("Free storage", or_dash(connected, format::bytes_as_gigabytes(status.number("/device_status/availableStorage")))))
            .when(!connected && !error.is_empty(), |readings| readings.child(muted(format!("Last error: {error}")).pt_2().text_size(px(12.5))))
    }
}

/// A card at `area` inside the dashboard.
fn placed(area: Area, card: AnyElement) -> impl IntoElement {
    div().absolute().left(px(area.left)).top(px(area.top)).w(px(area.width)).h(px(area.height)).child(card)
}

/// Card positions relative to the dashboard's top-left corner.
#[derive(Debug)]
struct DashboardLayout {
    network: Area,
    cpu: Area,
    gpu: Area,
    memory: Area,
    disk: Area,
}

impl DashboardLayout {
    /// KANALI's three columns with the network card spanning both rows; two or one column when narrow.
    fn fit(width: f32, row_height: f32) -> Self {
        let columns = ((width + CARD_GAP) / (MIN_COLUMN_WIDTH + CARD_GAP)).clamp(1.0, 3.0) as usize;
        let grid = Grid { column_width: (width - CARD_GAP * (columns - 1) as f32) / columns as f32, row_height };
        match columns {
            3 => Self { network: grid.span((0, 0), (1, 2)), cpu: grid.cell(1, 0), gpu: grid.cell(2, 0), memory: grid.cell(1, 1), disk: grid.cell(2, 1) },
            2 => Self { network: grid.span((0, 0), (2, 1)), cpu: grid.cell(0, 1), gpu: grid.cell(1, 1), memory: grid.cell(0, 2), disk: grid.cell(1, 2) },
            _ => Self { network: grid.cell(0, 0), cpu: grid.cell(0, 1), gpu: grid.cell(0, 2), memory: grid.cell(0, 3), disk: grid.cell(0, 4) },
        }
    }

    fn size(&self) -> Size<f32> {
        let all = [self.cpu, self.gpu, self.memory, self.disk].into_iter().fold(self.network, Area::union);
        size(all.right(), all.bottom())
    }
}

struct Grid {
    column_width: f32,
    row_height: f32,
}

impl Grid {
    fn cell(&self, column: usize, row: usize) -> Area {
        self.span((column, row), (1, 1))
    }

    fn span(&self, (column, row): (usize, usize), (columns, rows): (usize, usize)) -> Area {
        let left = column as f32 * (self.column_width + CARD_GAP);
        let top = row as f32 * (self.row_height + CARD_GAP);
        let width = columns as f32 * self.column_width + (columns - 1) as f32 * CARD_GAP;
        let height = rows as f32 * self.row_height + (rows - 1) as f32 * CARD_GAP;
        Area::new(left, top, width, height)
    }
}

/// Graphs side by side in a wide, short card; stacked otherwise.
fn graphs_side_by_side(width: f32, height: f32) -> bool {
    width > height * 1.6
}

struct NetworkSeries {
    name: &'static str,
    samples: Vec<f32>,
    color: u32,
    icon: &'static str,
}

impl NetworkSeries {
    fn peak_key(&self) -> &'static str {
        if self.name == "Download" { "Download peak" } else { "Upload peak" }
    }

    fn rate_key(&self) -> &'static str {
        if self.name == "Download" { "Download rate" } else { "Upload rate" }
    }
}

#[derive(Clone, Copy)]
enum Processor {
    Cpu,
    Gpu,
}

impl Processor {
    fn label(self) -> &'static str {
        match self {
            Processor::Cpu => "CPU Load",
            Processor::Gpu => "GPU Load",
        }
    }

    fn snapshot_key(self) -> &'static str {
        match self {
            Processor::Cpu => "cpu",
            Processor::Gpu => "gpu",
        }
    }
}

/// Icon and text in a card's bottom-left corner.
struct Footer {
    icon: &'static str,
    text: String,
}

struct LoadGauge {
    label: &'static str,
    load: f64,
    footer: Footer,
}

impl LoadGauge {
    fn read(status: &LiveStatus, processor: Processor, unit: &str) -> Self {
        let number = |field: &str| status.number(&format!("/snapshot/{}/{field}", processor.snapshot_key()));
        let footer = Footer { icon: assets::THERMOMETER, text: format::temperature(number("temperature"), unit) };
        Self { label: processor.label(), load: number("load"), footer }
    }
}

struct LoadBar {
    label: &'static str,
    load: f64,
    stops: &'static [u32],
    footer: Footer,
}

fn memory_bar(status: &LiveStatus) -> LoadBar {
    let usage = format::memory_usage(status.number("/snapshot/memory/used"), status.number("/snapshot/memory/total"));
    LoadBar { label: "Memory Load", load: status.number("/snapshot/memory/load"), stops: &MEMORY_GRADIENT, footer: Footer { icon: assets::MEMORY, text: usage } }
}

fn disk_bar(status: &LiveStatus) -> LoadBar {
    let usage = format::disk_usage(status.number("/snapshot/disk/used"), status.number("/snapshot/disk/total"));
    LoadBar { label: "Hard disk Load", load: status.number("/snapshot/disk/load"), stops: &DISK_GRADIENT, footer: Footer { icon: assets::DISK, text: usage } }
}

/// A full-height card topped by a big percentage and its label.
fn card_with_heading(load: f64, label: &'static str) -> Div {
    card().size_full().gap_1().child(bold(format::percent(load), 40.0)).child(muted(label).pb_2())
}

fn footer(footer: &Footer) -> impl IntoElement {
    div().flex().items_center().gap_2().h(px(26.0)).child(svg().path(footer.icon).size(px(20.0)).text_color(rgb(TEXT))).child(footer.text.clone())
}

fn device_identity(status: &LiveStatus) -> impl IntoElement {
    let line = |key: &'static str, pointer: &str| info_line(key, or_dash(true, status.text(pointer)));
    div()
        .flex()
        .flex_col()
        .child(line("Model", "/device/productId"))
        .child(info_line("Screen", "2240 × 1080"))
        .child(line("System", "/device/OS"))
        .child(line("Firmware", "/device/version/firmware"))
        .child(line("Hardware", "/device/version/hardware"))
        .child(line("Screen app", "/device/version/app"))
        .child(line("Serial number", "/device/sn"))
        .child(line("Serial port", "/port"))
}

fn computer_card(status: &LiveStatus) -> Div {
    card()
        .child(bold("Computer", 19.0))
        .child(info_line("Processor", or_dash(true, status.text("/spec/cpu"))))
        .child(info_line("Graphics", or_dash(true, status.text("/spec/gpu"))))
        .child(info_line("Memory", format::megabytes_as_gigabytes(status.number("/snapshot/memory/total"))))
        .child(info_line("System disk", format!("{:.0} GB", status.number("/snapshot/disk/total"))))
}

fn rpm(value: f64, connected: bool) -> String {
    or_dash(connected, format!("{value:.0} RPM"))
}

fn or_dash(show: bool, text: String) -> String {
    if show && !text.is_empty() { text } else { "—".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    // GPUI's own `test` attribute comes in with `gpui_kit::*`; these are plain tests.
    use core::prelude::v1::test;

    #[test]
    fn wide_dashboard_matches_kanali() {
        let layout = DashboardLayout::fit(940.0, 250.0);
        assert_eq!(layout.network, Area::new(0.0, 0.0, 300.0, 520.0));
        assert_eq!(layout.cpu, Area::new(320.0, 0.0, 300.0, 250.0));
        assert_eq!(layout.disk, Area::new(640.0, 270.0, 300.0, 250.0));
        assert_eq!(layout.size(), size(940.0, 520.0));
    }

    #[test]
    fn medium_dashboard_puts_network_across_the_top() {
        let layout = DashboardLayout::fit(600.0, 250.0);
        assert_eq!(layout.network, Area::new(0.0, 0.0, 600.0, 250.0));
        assert_eq!(layout.gpu, Area::new(310.0, 270.0, 290.0, 250.0));
        assert_eq!((layout.memory.left, layout.memory.top), (0.0, 540.0));
        assert_eq!(layout.size(), size(600.0, 790.0));
    }

    #[test]
    fn narrow_dashboard_stacks_every_card() {
        let layout = DashboardLayout::fit(400.0, 250.0);
        assert_eq!(layout.disk, Area::new(0.0, 1080.0, 400.0, 250.0));
        assert_eq!(layout.size(), size(400.0, 1330.0));
    }

    #[test]
    fn network_graphs_sit_side_by_side_only_in_wide_cards() {
        assert!(graphs_side_by_side(500.0, 150.0));
        assert!(!graphs_side_by_side(250.0, 420.0));
    }
}
