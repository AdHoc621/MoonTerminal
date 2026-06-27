//! Форматирование чисел для UI/feed.

/// Компактное число: точность `decimals`, хвостовые нули и точка срезаются
/// ("1.500000" → "1.5", "2.000000" → "2").
pub fn compact(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Компактное число с SI-суффиксом (K/M/B/T): 1_500 → «1.5K», 2_300_000 → «2.3M».
/// Значения меньше 1000 идут через [`adaptive`] (без суффикса). Хвостовые нули срезаются.
pub fn compact_si(v: f64) -> String {
    let a = v.abs();
    if a < 1000.0 {
        return adaptive(v);
    }
    const UNITS: [(f64, &str); 4] = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "K")];
    for (scale, suffix) in UNITS {
        if a >= scale {
            let n = v / scale;
            let s = if n.abs() >= 100.0 {
                format!("{n:.0}")
            } else if n.abs() >= 10.0 {
                format!("{n:.1}")
            } else {
                format!("{n:.2}")
            };
            let s = s.trim_end_matches('0').trim_end_matches('.');
            return format!("{s}{suffix}");
        }
    }
    adaptive(v)
}

/// Адаптивное число под размер/цену: точность подбирается по величине, а не фиксирована.
/// Крупные значения — без дробной части (5000000.0001 → "5000000", 5000 → "5000");
/// мелкие — с достаточным числом знаков, чтобы значащие цифры были видны
/// (0.0000001 → "0.0000001"). `sig` — желаемое число значащих цифр (для дробной части).
pub fn adaptive(v: f64) -> String {
    let a = v.abs();
    if a == 0.0 {
        return "0".to_string();
    }
    // Тысячи и больше — без дробной части.
    if a >= 1000.0 {
        return compact(v, 0);
    }
    const SIG: i32 = 5;
    // Экспонента старшего разряда: для a<1 отрицательна (0.0001 → -4).
    let exp = a.log10().floor() as i32;
    // Знаков после запятой = столько, чтобы набрать SIG значащих цифр (с запасом
    // на ведущие нули у мелких чисел). Ограничиваем сверху на разумный максимум.
    let decimals = (SIG - 1 - exp).clamp(0, 18) as usize;
    compact(v, decimals)
}
