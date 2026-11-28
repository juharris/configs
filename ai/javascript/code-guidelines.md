# JavaScript Code Guidelines

## Dates and Times for People

In a web page, show each date and time in the browser's own field, styled as text.
The browser chooses how to show it to the reader.

| To show | Field | Set |
|---|---|---|
| A date | `<input type="date" inert>` | `valueAsNumber` to `Date.UTC(time.getFullYear(), time.getMonth(), time.getDate())` |
| A time of day | `<input type="time" step="1" inert>` | `value` to `time.toTimeString().slice(0, 8)` |
| A date with a time | `<input type="datetime-local" step="1" inert>` | `valueAsNumber` to `time.getTime() - time.getTimezoneOffset() * 60_000` |

Build each field in one function that takes a `Date`, so that callers never touch the field.

```ts
function createFormattedTimeOfDay(time: Date): HTMLTimeElement {
  const element = document.createElement("time");
  const field = document.createElement("input");
  const text = document.createElement("span");

  field.inert = true;
  field.step = "1";
  field.type = "time";
  field.value = time.toTimeString().slice(0, 8);
  // An inert field has no text, so screen readers and copied text get its value, which has one format for every reader.
  text.className = "visually-hidden";
  text.textContent = field.value;
  element.dateTime = time.toISOString();
  element.append(text, field);
  return element;
}
```

- `inert` keeps the fields out of the tab order, which matters when a table or chart holds many of them.
  For one field that the reader should reach, such as a clock, use `readonly` with an `aria-label` and leave out the hidden text.
- Style the field like the text around it: no background, border, or padding, the inherited color, and `::-webkit-calendar-picker-indicator { display: none; }`.
- Inside an SVG, put the field in a `foreignObject`.
- In tests, pin the time zone and check the field's `value`, which is the same for every display format.

### Why

Web JavaScript has no portable way to get a plain string that honors the operating system's date and time format.
`Intl` and the `toLocale*()` methods follow only the browser's language, and `toString()` prints the same English layout for every reader.
Chrome on macOS shows the fields in the system's format when Chrome's own language matches the system language, as [Chromium's macOS locale selection](https://github.com/chromium/chromium/blob/main/third_party/blink/renderer/platform/text/locale_mac.mm#L49-L76) shows.
The page's `lang` does not change the format.
Other browsers may use a locale default, as [HTML form control localization](https://html.spec.whatwg.org/multipage/input.html#input-impl-notes) allows.

Verified on 2026-11-27 in Chromium 149 on macOS set to a 24-hour clock and `2026-11-27` dates: `new Date().toLocaleTimeString()` showed `3:46:10 PM`, and the time field showed `15:46:10`.
