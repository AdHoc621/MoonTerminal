//! Таблица панели «Ордера»: колонки, строки/ячейки, клик по токену, тогл стопов.

use super::*;
use moon_core::feed::OrderStopKind;
use rust_i18n::t;

pub(super) fn orders_table(
    rows: Rc<Vec<OrderEntry>>,
    columns: u16,
    cx: &Context<OrdersPanel>,
) -> impl IntoElement {
    let empty = rows.is_empty();
    let row_count = rows.len();
    let view = cx.entity();
    let table_rows = rows.clone();
    let p = MoonPalette::active(cx);
    // Видимые колонки в каноничном порядке — общий список для header и строк.
    let visible: Rc<Vec<OrdCol>> = Rc::new(
        OrdCol::ALL
            .iter()
            .copied()
            .filter(|c| columns & c.bit() != 0)
            .collect(),
    );
    let row_cols = visible.clone();

    crate::panels::common::data_table_host(
        "orders-table-host",
        empty,
        t!("orders.empty").to_string(),
        p,
        cx,
        MoonDataTable::new("orders-table", row_count, move |ix, _window, _app| {
            order_table_row(&table_rows[ix], &view, p, &row_cols)
        })
        .columns(visible.iter().map(|c| column_def(*c)).collect::<Vec<_>>())
        .header_height(design::TABLE_HEAD_H)
        .row_height(design::TABLE_ROW_H),
    )
}

/// Переводимый/отраслевой заголовок колонки. Core/Side/Token/Cur.P идут через словарь
/// `orders.col.*`; Size/SL/TS/Vstop/Buy/Fill/Strat — отраслевые токены, намеренно НЕ
/// переводим (см. locales/README.md). Общий для header и меню выбора полей.
pub(super) fn col_title(col: OrdCol) -> String {
    match col {
        OrdCol::Core => t!("orders.col.core").to_string(),
        OrdCol::Side => t!("orders.col.side").to_string(),
        OrdCol::Token => t!("orders.col.token").to_string(),
        OrdCol::CurP => t!("orders.col.price").to_string(),
        OrdCol::Size => "Size".to_string(),
        OrdCol::Sl => "SL".to_string(),
        OrdCol::Ts => "TS".to_string(),
        OrdCol::Vstop => "Vstop".to_string(),
        OrdCol::Buy => "Buy".to_string(),
        OrdCol::Fill => "Fill".to_string(),
        OrdCol::Strat => "Strat".to_string(),
    }
}

/// Схема колонки: ключ/ширина/выравнивание. Порядок задаётся `OrdCol::ALL`. Ширина —
/// логические px (минимум на узкой таблице, пропорциональный вес на широкой).
fn column_def(col: OrdCol) -> MoonDataTableColumn {
    let title = col_title(col);
    match col {
        OrdCol::Core => MoonDataTableColumn::new("core", title, 90.0),
        OrdCol::Side => MoonDataTableColumn::new("side", title, 60.0),
        OrdCol::Token => numeric_column("token", title, 70.0),
        OrdCol::Size => numeric_column("size", title, 70.0),
        OrdCol::Sl => MoonDataTableColumn::new("sl", title, 46.0),
        OrdCol::Ts => MoonDataTableColumn::new("ts", title, 46.0),
        OrdCol::Vstop => MoonDataTableColumn::new("vstop", title, 56.0),
        OrdCol::Buy => numeric_column("buy", title, 80.0),
        OrdCol::CurP => numeric_column("cur.p", title, 86.0),
        OrdCol::Fill => numeric_column("fill", title, 56.0),
        OrdCol::Strat => numeric_column("strat", title, 90.0),
    }
}

fn numeric_column(
    key: impl Into<SharedString>,
    title: impl Into<SharedString>,
    width: f32,
) -> MoonDataTableColumn {
    MoonDataTableColumn::new(key, title, width).right()
}

fn order_table_row(
    e: &OrderEntry,
    view: &Entity<OrdersPanel>,
    p: MoonPalette,
    cols: &[OrdCol],
) -> MoonDataRow {
    MoonDataRow::new(cols.iter().map(|c| cell_for(*c, e, view, p)).collect::<Vec<_>>())
}

/// Ячейка для одной колонки строки. Порядок ячеек ДОЛЖЕН совпадать с `column_def` по тем
/// же видимым колонкам — оба идут по одному списку `cols`.
fn cell_for(col: OrdCol, e: &OrderEntry, view: &Entity<OrdersPanel>, p: MoonPalette) -> MoonDataCell {
    let r = &e.row;
    match col {
        OrdCol::Core => MoonDataCell::text(e.core_name.clone()).tone(MoonTone::Muted),
        OrdCol::Side => {
            let (side, tone) = side_label(r);
            MoonDataCell::text(side).tone(tone).weight(500.0)
        }
        OrdCol::Token => MoonDataCell::element(token_cell(e, view, p)),
        OrdCol::Size => MoonDataCell::text(num(r.size)),
        OrdCol::Sl => flag_toggle_cell(e, view, OrderStopKind::StopLoss, r.sl_on, p),
        OrdCol::Ts => flag_toggle_cell(e, view, OrderStopKind::Trailing, r.ts_on, p),
        OrdCol::Vstop => flag_toggle_cell(e, view, OrderStopKind::VStop, r.vstop_on, p),
        OrdCol::Buy => MoonDataCell::text(num(r.buy_price)),
        OrdCol::CurP => MoonDataCell::text(num(r.price as f64)),
        OrdCol::Fill => MoonDataCell::text(format!("{:.0}%", r.fill_pct)).tone(MoonTone::Muted),
        OrdCol::Strat => MoonDataCell::text(r.strat.clone()).tone(MoonTone::Muted),
    }
}

/// Отображаемая сторона и её тон: SELL (исполненный лонг) — синим, SHORT — красным,
/// BUY (ждёт) — зелёным; эмулятор → суффикс `(E)`.
fn side_label(r: &OrderRow) -> (String, MoonTone) {
    let (side, tone) = if is_sell(r) {
        ("SELL", MoonTone::Info)
    } else if r.is_short {
        ("SHORT", MoonTone::Danger)
    } else {
        ("BUY", MoonTone::Positive)
    };
    let side = if r.emulator {
        format!("{side}(E)")
    } else {
        side.to_string()
    };
    (side, tone)
}

/// Кликабельный флаг стопа (SL/TS/Vstop): ON — зелёным, OFF — тускло. Клик шлёт ядру
/// `set_order_stop` (включить/выключить ИНВЕРСИЕЙ текущего флага) для ЭТОГО ордера —
/// уровень стопа сохраняется feed-слоем при повторном включении.
fn flag_toggle_cell(
    e: &OrderEntry,
    view: &Entity<OrdersPanel>,
    kind: OrderStopKind,
    on: bool,
    p: MoonPalette,
) -> MoonDataCell {
    let core = e.core;
    let uid = e.row.uid;
    let view = view.clone();
    let (label, tone) = if on {
        ("ON", MoonTone::Positive)
    } else {
        ("OFF", MoonTone::Muted)
    };
    let key = match kind {
        OrderStopKind::StopLoss => "sl",
        OrderStopKind::Trailing => "ts",
        OrderStopKind::VStop => "vs",
    };
    let el = div()
        .id(SharedString::from(format!("ord-{key}-{core}-{uid}")))
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .cursor_pointer()
        .child(
            MoonText::new(label)
                .color(tone.color(p))
                .font_size(10.5)
                .line_height(14.0)
                .weight(500.0)
                .mono(true)
                .uppercase(false)
                .render(),
        )
        .on_click(move |_, _window, app| {
            log::info!("orders UI click toggle stop core={core} uid={uid} kind={kind:?} on={on} -> {}", !on);
            view.update(app, |this, cx| {
                this.backend.update(cx, |b, _| {
                    if let Err(err) = b.session.set_order_stop(core, uid, kind, !on) {
                        log::warn!(
                            "orders toggle stop failed core={core} uid={uid} kind={kind:?}: {err:#}"
                        );
                    }
                });
                cx.notify();
            });
        });
    MoonDataCell::element(el)
}

/// Ячейка токена (без quote: `ADAUSDT` → `ADA`), акцентом — намёк, что кликабельна.
/// Клик открывает чарт монеты на Main НА ЯДРЕ ордера (порт клика по строке egui).
fn token_cell(
    e: &OrderEntry,
    view: &Entity<OrdersPanel>,
    p: MoonPalette,
) -> impl IntoElement + 'static {
    let token = symbol::base_symbol(&e.row.market, &e.quote).to_string();
    let core = e.core;
    let market = e.row.market.clone();
    let uid = e.row.uid;
    let view = view.clone();

    div()
        .id(SharedString::from(format!("ord-tok-{core}-{uid}")))
        .h_full()
        .flex()
        .items_center()
        .cursor_pointer()
        .child(
            MoonText::new(token)
                .color(MoonTone::Accent.color(p))
                .font_size(10.5)
                .line_height(14.0)
                .weight(500.0)
                .mono(true)
                .uppercase(false)
                .render(),
        )
        .on_click(move |_, _window, app| {
            view.update(app, |this, cx| {
                this.backend.update(cx, |b, bcx| {
                    b.open_request = Some((core, market.clone()));
                    b.open_request_rev = b.open_request_rev.wrapping_add(1);
                    // Клик в Ордерах открывает монету на Main, но окно НЕ поднимает.
                    b.open_request_activate = false;
                    bcx.notify();
                });
            });
        })
}

/// Открытые ордера всех ядер группы — для статус-бара Shell (число ордеров).
pub fn count_orders(b: &Backend, group: &str) -> usize {
    let store = b.session.store();
    b.session
        .sessions()
        .iter()
        .filter(|s| s.group == group)
        .filter_map(|s| store.core(s.id))
        .map(|c| c.orders.len())
        .sum()
}
