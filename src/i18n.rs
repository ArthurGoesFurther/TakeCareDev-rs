//! Translations (ru, en). `tr(lang, key, args)` falls back to English, then to the key itself.

/// (key, ru, en)
const TABLE: &[(&str, &str, &str)] = &[
    ("appName", "TakeCareDev", "TakeCareDev"),
    ("ex.leftRight.title", "Влево — вправо", "Left — right"),
    ("ex.leftRight.hint", "Води глазами за моими зрачками. Голова неподвижна.", "Follow my pupils with your eyes. Keep your head still."),
    ("ex.upDown.title", "Вверх — вниз", "Up — down"),
    ("ex.upDown.hint", "Смотри вверх и вниз, не двигая головой.", "Look up and down without moving your head."),
    ("ex.circles.title", "Круги", "Circles"),
    ("ex.circles.hint", "Медленно вращай глазами по кругу, потом в обратную сторону.", "Slowly roll your eyes in a circle, then the other way."),
    ("ex.diagonals.title", "Диагонали", "Diagonals"),
    ("ex.diagonals.hint", "Смотри из угла в угол: сначала одна диагональ, потом другая.", "Look corner to corner: one diagonal, then the other."),
    ("ex.figureEight.title", "Восьмёрка", "Figure eight"),
    ("ex.figureEight.hint", "Рисуй глазами лежащую восьмёрку ∞.", "Trace a sideways eight ∞ with your eyes."),
    ("ex.blink.title", "Поморгай", "Blink"),
    ("ex.blink.hint", "Часто и легко моргай — так глаза увлажняются.", "Blink quickly and lightly — it keeps your eyes moist."),
    ("ex.squeeze.title", "Зажмурься", "Squeeze"),
    ("ex.squeeze.hint", "Крепко зажмурься на пару секунд, потом широко открой глаза.", "Squeeze your eyes shut for a couple of seconds, then open them wide."),
    ("ex.farAway.title", "Вдаль", "Far away"),
    ("ex.farAway.hint", "Посмотри в окно или на самый дальний предмет (от 6 метров).", "Look out of the window or at the farthest object (6+ m away)."),
    ("ex.nearFar.title", "Близко — далеко", "Near — far"),
    ("ex.nearFar.hint", "Посмотри на кончик носа, потом вдаль. Чередуй.", "Look at the tip of your nose, then far away. Alternate."),
    ("ex.palming.title", "Отдых", "Rest"),
    ("ex.palming.hint", "Закрой глаза и расслабься. Можно накрыть их тёплыми ладонями.", "Close your eyes and relax. You can cover them with warm palms."),
    ("time.min", "{0} мин", "{0} min"),
    ("time.sec", "{0} с", "{0} s"),
    ("time.lessMin", "< 1 мин", "< 1 min"),
    ("tray.eyesIn", "Глаза: через {0}", "Eyes: in {0}"),
    ("tray.walkIn", "Перерыв: через {0}", "Break: in {0}"),
    ("tray.eyesOff", "Глаза: выключено", "Eyes: off"),
    ("tray.walkOff", "Перерыв: выключен", "Break: off"),
    ("tray.pausedUntil", "На паузе до {0}", "Paused until {0}"),
    ("tray.paused", "На паузе", "Paused"),
    ("tray.today", "Сегодня: глаза {0} · перерывы {1}", "Today: eyes {0} · breaks {1}"),
    ("tray.eyeNow", "Упражнение для глаз сейчас", "Eye exercise now"),
    ("tray.walkNow", "Перерыв сейчас", "Take a break now"),
    ("tray.skipBreak", "Пропустить перерыв", "Skip the break"),
    ("tray.pause", "Пауза", "Pause"),
    ("tray.pause30", "30 минут", "30 minutes"),
    ("tray.pause60", "1 час", "1 hour"),
    ("tray.pause120", "2 часа", "2 hours"),
    ("tray.pauseForever", "Пока не возобновлю", "Until I resume"),
    ("tray.resume", "Возобновить", "Resume"),
    ("tray.reset", "Сбросить таймеры", "Reset timers"),
    ("tray.settings", "Настройки…", "Settings…"),
    ("tray.quit", "Выход", "Quit"),
    ("notify.walkSoonTitle", "Скоро перерыв", "Break soon"),
    ("notify.walkSoonBody", "Через {0} — время встать и походить.", "In {0} — time to stand up and walk."),
    ("notify.runningTitle", "TakeCareDev работает", "TakeCareDev is running"),
    ("notify.runningBody", "Живу в трее. Настройки — там же или через повторный запуск.", "I live in the tray. Settings are there, or just launch me again."),
    ("break.title", "Время размяться!", "Time to move!"),
    ("break.status.running", "Иди-иди, я посторожу", "Off you go, I will keep watch"),
    ("break.status.waiting", "Таймер ждёт, пока ты отойдёшь от компьютера", "The timer waits until you step away from the computer"),
    ("break.status.plain", "Встань и подвигайся, пока идёт таймер", "Stand up and move while the timer runs"),
    ("break.doneTitle", "Готово!", "Done!"),
    ("break.doneText", "Можно возвращаться к работе.", "You can get back to work."),
    ("break.snooze", "Отложить на {0}", "Snooze {0}"),
    ("break.skip", "Пропустить", "Skip"),
    ("break.close", "Закрыть", "Close"),
    ("break.strictNote", "Строгий режим. Экстренно пропустить можно через меню в трее.", "Strict mode. Emergency skip is in the tray menu."),
    ("set.title", "TakeCareDev — настройки", "TakeCareDev — settings"),
    ("set.statusEyes", "Глаза через {0}", "Eyes in {0}"),
    ("set.statusWalk", "перерыв через {0}", "break in {0}"),
    ("set.statusOff", "выкл.", "off"),
    ("set.statusBusy", "сейчас", "now"),
    ("set.stats", "Сегодня: упражнений {0}, перерывов {1}, пропущено {2}", "Today: exercises {0}, breaks {1}, skipped {2}"),
    ("set.section.eyes", "Гимнастика для глаз", "Eye exercises"),
    ("set.section.walk", "Перерыв: встать и походить", "Break: stand up and walk"),
    ("set.section.general", "Общее", "General"),
    ("set.enabled", "Включено", "Enabled"),
    ("set.eyes.intervalMin", "Каждые, мин", "Every, min"),
    ("set.eyes.durationSec", "Длительность упражнения, сек", "Exercise duration, sec"),
    ("set.eyes.position", "Где показывать лицо", "Where to show the face"),
    ("set.eyes.followCursor", "Следовать за курсором", "Follow the cursor"),
    ("set.eyes.faceScale", "Размер лица", "Face size"),
    ("set.eyes.showHint", "Подсказка рядом с лицом", "Hint bubble next to the face"),
    ("set.eyes.order", "Порядок упражнений", "Exercise order"),
    ("set.eyes.sound", "Звук", "Sound"),
    ("set.eyes.exercises", "Упражнения", "Exercises"),
    ("set.eyes.exercisesNote", "Нажми на название — лицо в шапке покажет, кнопка «играть» — покажет у курсора.", "Click a name to preview it in the header; the play button shows it next to the cursor."),
    ("set.walk.intervalMin", "Каждые, мин", "Every, min"),
    ("set.walk.durationMin", "Длительность перерыва, мин", "Break duration, min"),
    ("set.walk.snoozeMin", "«Отложить» на, мин", "“Snooze” for, min"),
    ("set.walk.warnSec", "Предупредить заранее, сек (0 — не надо)", "Warn in advance, sec (0 — no)"),
    ("set.walk.requireIdle", "Засчитывать только время вдали от компьютера", "Count only time away from the computer"),
    ("set.walk.strict", "Строгий режим: на весь экран и без «Пропустить»", "Strict mode: full screen, no “Skip”"),
    ("set.walk.sound", "Звук", "Sound"),
    ("set.idle.resetAfterMin", "Если меня нет дольше, мин — начать отсчёт заново (0 — выкл.)", "Restart the countdown after being away for, min (0 — off)"),
    ("set.language", "Язык", "Language"),
    ("set.autostart", "Запускать при входе в систему", "Launch at login"),
    ("set.pos.cursor", "Возле курсора", "Next to the cursor"),
    ("set.pos.top-left", "Слева сверху", "Top left"),
    ("set.pos.top-right", "Справа сверху", "Top right"),
    ("set.pos.bottom-left", "Слева снизу", "Bottom left"),
    ("set.pos.bottom-right", "Справа снизу", "Bottom right"),
    ("set.pos.center", "По центру", "Center"),
    ("set.order.random", "Случайно", "Random"),
    ("set.order.sequential", "По очереди", "In turn"),
    ("set.lang.auto", "Как в системе", "System"),
    ("set.btn.eyeNow", "Показать сейчас", "Show now"),
    ("set.btn.walkNow", "Перерыв сейчас", "Break now"),
    ("set.btn.reset", "Сбросить таймеры", "Reset timers"),
    ("set.btn.pause", "Пауза 30 мин", "Pause 30 min"),
    ("set.btn.resume", "Возобновить", "Resume"),
    ("set.note.waylandFollow", "Wayland не даёт приложениям следить за курсором: лицо появится рядом с ним, но догонять не будет.", "Wayland does not let apps track the cursor: the face appears next to it but will not follow."),
    ("set.note.noCursor", "В этой сессии позиция курсора недоступна — лицо будет в правом нижнем углу.", "The cursor position is unavailable in this session — the face goes to the bottom right corner."),
    ("set.note.idleUnreliable", "Здесь не получается узнать время простоя, поэтому сброс по отсутствию и «только вдали от компьютера» не работают.", "Idle time is unavailable here, so the away reset and “only away from the computer” are off."),
];

const TIPS_RU: [&str; 8] = [
    "Встань и пройдись — хотя бы до кухни и обратно.",
    "Пройдись по лестнице пару пролётов.",
    "Сделай 10 приседаний.",
    "Потянись вверх, покрути плечами и шеей.",
    "Сделай наклоны в стороны и вперёд.",
    "Налей себе воды и выпей её стоя.",
    "Походи на месте, высоко поднимая колени.",
    "Пройдись и посмотри в окно вдаль.",
];
const TIPS_EN: [&str; 8] = [
    "Stand up and walk — at least to the kitchen and back.",
    "Take the stairs for a couple of flights.",
    "Do 10 squats.",
    "Stretch up, roll your shoulders and neck.",
    "Bend to the sides and forward.",
    "Pour yourself some water and drink it standing.",
    "March in place, knees up high.",
    "Walk around and look far out of the window.",
];

pub fn tip(lang: &str, n: usize) -> &'static str {
    let tips = if lang == "ru" { &TIPS_RU } else { &TIPS_EN };
    tips[n % tips.len()]
}

/// `setting` is auto | ru | en; `locale` looks like "ru_RU.UTF-8".
pub fn resolve_language(setting: &str, locale: &str) -> &'static str {
    match setting {
        "ru" => "ru",
        "en" => "en",
        _ => {
            let code = locale.split(['-', '_', '.']).next().unwrap_or("").to_lowercase();
            if ["ru", "be", "kk", "ky"].contains(&code.as_str()) { "ru" } else { "en" }
        }
    }
}

/// Language of the OS UI, from the usual POSIX variables.
pub fn system_locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

pub fn tr(lang: &str, key: &str, args: &[&dyn ToString]) -> String {
    let Some(&(_, ru, en)) = TABLE.iter().find(|(k, _, _)| *k == key) else {
        return key.to_string();
    };
    let mut text = if lang == "ru" { ru } else { en }.to_string();
    for (i, arg) in args.iter().enumerate() {
        text = text.replace(&format!("{{{i}}}"), &arg.to_string());
    }
    text
}

pub fn format_left(lang: &str, sec: u64) -> String {
    if sec < 60 { tr(lang, "time.lessMin", &[]) } else { tr(lang, "time.min", &[&sec.div_ceil(60)]) }
}

pub fn format_clock(sec: f64) -> String {
    let s = sec.max(0.0).ceil() as u64;
    let (h, m) = (s / 3600, s % 3600 / 60);
    if h > 0 { format!("{h}:{m:02}:{:02}", s % 60) } else { format!("{m:02}:{:02}", s % 60) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_and_formatting() {
        assert_eq!(resolve_language("auto", "ru_RU.UTF-8"), "ru");
        assert_eq!(resolve_language("auto", "de_DE"), "en");
        assert_eq!(resolve_language("en", "ru"), "en");
        assert_eq!(tr("ru", "time.min", &[&5]), "5 мин");
        assert_eq!(tr("en", "nope", &[]), "nope");
        assert_eq!(format_clock(59.2), "01:00");
        assert_eq!(format_clock(3725.0), "1:02:05");
        assert_eq!(format_left("en", 61), "2 min");
    }
}
