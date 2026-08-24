use cef::rc::Rc as _;
use cef::{
    CefString, Frame, ImplFrame, ImplListValue, ImplProcessMessage, ImplRenderProcessHandler,
    ImplV8Context, ImplV8Exception, ImplV8Handler, ImplV8Value, ProcessId, ProcessMessage,
    RenderProcessHandler, V8Context, V8Handler, V8Value, WrapRenderProcessHandler, WrapV8Handler,
    process_message_create, v8_context_get_current_context, v8_value_create_function,
    wrap_render_process_handler, wrap_v8_handler,
};
use gpui::{Hsla, Rgba};
use serde::Deserialize;

use crate::text_input::send_text_input_state;

pub(crate) const PAGE_CHROME_MESSAGE_NAME: &str = "qedit.page_chrome";
const PAGE_CHROME_BRIDGE_NAME: &str = "__qeditReportChromeColor";

// Failure modes:
// - The top edge may be transparent because the page paints with gradients, images, or video.
// - The DOM might not be ready when the render context is created.
// - Pages can mutate the header after load or change it on scroll.
// - Navigation can leave a stale color behind if the next page never reports one.
//
// Strategy:
// - Sample what is actually visible at the top edge of the viewport.
// - Fall back to <meta name="theme-color"> when sampling finds no solid color.
// - Install observers for load, resize, scroll, and DOM mutations.
// - Let the browser process clear the stored color on navigation boundaries.
const PAGE_CHROME_OBSERVER_SCRIPT: &str = r#"
(function () {
  if (window.__qeditChromeSyncInstalled) return;
  window.__qeditChromeSyncInstalled = true;

  const bridge = window.__qeditReportChromeColor;
  if (typeof bridge !== 'function') return;

  let lastPayload = null;
  let scheduled = false;

  const isTransparent = (color) => {
    if (!color) return true;
    const normalized = color.replace(/\s+/g, '').toLowerCase();
    return normalized === 'transparent' || normalized === 'rgba(0,0,0,0)';
  };

  const parseThemeMeta = () => {
    const meta = document.querySelector('meta[name="theme-color" i]');
    const content = meta && typeof meta.content === 'string' ? meta.content.trim() : '';
    return content || null;
  };

  const parseAlpha = (color) => {
    if (!color) return 0;
    const match = color
      .replace(/\s+/g, '')
      .match(/^rgba?\(([\d.]+),([\d.]+),([\d.]+)(?:,([\d.]+))?\)$/i);
    if (!match) return 1;
    return match[4] === undefined ? 1 : Math.max(0, Math.min(1, Number(match[4])));
  };

  const solidBackgroundFor = (element) => {
    let current = element;
    while (current && current.nodeType === Node.ELEMENT_NODE) {
      const style = window.getComputedStyle(current);
      const color = style && style.backgroundColor;
      if (!isTransparent(color)) {
        return {
          color,
          alpha: parseAlpha(color),
          hasBackdrop:
            (style.backdropFilter && style.backdropFilter !== 'none') ||
            (style.webkitBackdropFilter && style.webkitBackdropFilter !== 'none'),
          rect: current.getBoundingClientRect(),
        };
      }
      current = current.parentElement;
    }
    return null;
  };

  const sampleAt = (x, y) => {
    const elements = document.elementsFromPoint(x, y);
    let fallback = null;
    for (const element of elements) {
      const candidate = solidBackgroundFor(element);
      if (!candidate) continue;

      const widthFraction = candidate.rect.width / Math.max(1, window.innerWidth);
      const looksLikeOverlay =
        candidate.hasBackdrop || candidate.alpha < 0.98 || widthFraction < 0.7;
      const resolved = {
        color: candidate.color,
        looksLikeOverlay,
        widthFraction,
      };

      if (!looksLikeOverlay) {
        return resolved;
      }

      fallback ||= resolved;
    }

    return fallback;
  };

  const colorMetrics = (color) => {
    const match = color
      .replace(/\s+/g, '')
      .match(/^rgba?\(([\d.]+),([\d.]+),([\d.]+)(?:,([\d.]+))?\)$/i);
    if (!match) return null;
    const r = Number(match[1]);
    const g = Number(match[2]);
    const b = Number(match[3]);
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    return {
      average: (r + g + b) / 3,
      saturation: max === 0 ? 0 : (max - min) / max,
    };
  };

  const isWeakNeutralColor = (color) => {
    const metrics = colorMetrics(color);
    return metrics ? metrics.average > 242 && metrics.saturation < 0.08 : false;
  };

  const bandAt = (xPoints, width, y) => {
    const counts = new Map();
    for (const x of xPoints) {
      const sample = sampleAt(Math.min(Math.max(1, x), width - 1), y);
      if (!sample) continue;

      const key = sample.color;
      const entry = counts.get(key) || {
        color: sample.color,
        count: 0,
        overlayCount: 0,
        widthTotal: 0,
      };
      entry.count += 1;
      entry.overlayCount += sample.looksLikeOverlay ? 1 : 0;
      entry.widthTotal += sample.widthFraction;
      counts.set(key, entry);
    }

    let winner = null;
    for (const entry of counts.values()) {
      if (
        !winner ||
        entry.count > winner.count ||
        (entry.count === winner.count && entry.overlayCount < winner.overlayCount) ||
        (
          entry.count === winner.count &&
          entry.overlayCount === winner.overlayCount &&
          entry.widthTotal > winner.widthTotal
        )
      ) {
        winner = entry;
      }
    }

    if (!winner) return null;

    return {
      ...winner,
      averageWidthFraction: winner.widthTotal / winner.count,
    };
  };

  const isStrongTopHeaderBand = (band, y) => {
    return (
      y <= 28 &&
      band.count >= 2 &&
      band.overlayCount === 0 &&
      band.averageWidthFraction >= 0.9 &&
      !isWeakNeutralColor(band.color)
    );
  };

  const scoreBand = (band, y, totalSamples) => {
    const metrics = colorMetrics(band.color);
    const dominance = band.count / Math.max(1, totalSamples);
    const solidFraction = (band.count - band.overlayCount) / Math.max(1, band.count);
    const topBias = Math.max(0, 1 - y / 160);
    let score =
      dominance * 4 +
      solidFraction * 3 +
      band.averageWidthFraction * 2 +
      topBias * 1.5;

    if (metrics) {
      score += Math.max(0, (235 - metrics.average) / 235) * 1.5;
      score += Math.min(1, metrics.saturation * 2);

      if (metrics.average > 246 && metrics.saturation < 0.05) {
        score -= 5;
      } else if (metrics.average > 238 && metrics.saturation < 0.08) {
        score -= 2;
      }
    }

    return score;
  };

  const pickChromeColor = () => {
    const width = Math.max(1, window.innerWidth);
    const xPoints = [
      Math.floor(width * 0.2),
      Math.floor(width * 0.5),
      Math.floor(width * 0.8),
    ];
    const yPoints = [6, 16, 28, 44, 64, 88, 116, 148]
      .map((value) => Math.min(Math.max(1, value), Math.max(1, window.innerHeight - 1)));

    let firstResolved = null;
    let bestBand = null;

    for (const y of yPoints) {
      const band = bandAt(xPoints, width, y);
      if (!band) continue;
      firstResolved ||= band.color;

      if (isStrongTopHeaderBand(band, y)) {
        return band.color;
      }

      const score = scoreBand(band, y, xPoints.length);
      if (!bestBand || score > bestBand.score) {
        bestBand = { color: band.color, score };
      }
    }

    return bestBand ? bestBand.color : firstResolved;
  };

  const report = () => {
    scheduled = false;

    let source = 'sampled_top_edge';
    let color = pickChromeColor();
    if (!color) {
      color = parseThemeMeta();
      source = color ? 'theme_color_meta' : 'none';
    }

    const payload = color ? JSON.stringify({ color, source }) : '';
    if (payload === lastPayload) return;
    lastPayload = payload;
    bridge(payload);
  };

  const schedule = () => {
    if (scheduled) return;
    scheduled = true;
    window.requestAnimationFrame(report);
  };

  const installObservers = () => {
    const root = document.documentElement;
    if (!root) {
      document.addEventListener('DOMContentLoaded', installObservers, { once: true });
      return;
    }

    new MutationObserver(schedule).observe(root, {
      attributes: true,
      childList: true,
      subtree: true,
      attributeFilter: ['class', 'style', 'content'],
    });

    window.addEventListener('scroll', schedule, { passive: true, capture: true });
    window.addEventListener('resize', schedule, { passive: true });
    window.addEventListener('focus', schedule, { passive: true });
    window.addEventListener('pageshow', schedule, { passive: true });
    window.addEventListener('load', schedule, { once: true });
    document.addEventListener('DOMContentLoaded', schedule, { once: true });
    document.addEventListener('visibilitychange', schedule);

    // Some pages finish painting the hero/header after the initial DOM and load callbacks.
    // Re-sampling shortly after install avoids requiring user interaction to settle the color.
    window.setTimeout(schedule, 120);
    window.setTimeout(schedule, 350);

    schedule();
  };

  installObservers();
})();
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PageChromeSource {
    SampledTopEdge,
    ThemeColorMeta,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PageChrome {
    pub(crate) color: Hsla,
    pub(crate) source: PageChromeSource,
}

#[derive(Debug, Deserialize)]
struct PageChromePayload {
    color: String,
    source: PageChromeSource,
}

pub(crate) fn parse_page_chrome_payload(payload: &str) -> Option<PageChrome> {
    if payload.trim().is_empty() {
        return None;
    }

    let payload: PageChromePayload = serde_json::from_str(payload).ok()?;
    let color = parse_css_color(&payload.color)?;
    Some(PageChrome {
        color,
        source: payload.source,
    })
}

pub(crate) fn extract_page_chrome_from_message(
    message: &mut ProcessMessage,
) -> Option<Option<PageChrome>> {
    if CefString::from(&message.name()).to_string() != PAGE_CHROME_MESSAGE_NAME {
        return None;
    }

    let args = message.argument_list()?;
    let payload = CefString::from(&args.string(0)).to_string();
    if payload.trim().is_empty() {
        return Some(None);
    }

    parse_page_chrome_payload(&payload).map(Some)
}

#[derive(Clone)]
struct PageChromeBridgeV8Handler;

wrap_v8_handler! {
    struct PageChromeBridgeV8HandlerBuilder {
        handler: PageChromeBridgeV8Handler,
    }

    impl V8Handler {
        fn execute(
            &self,
            _name: Option<&CefString>,
            _object: Option<&mut V8Value>,
            arguments: Option<&[Option<V8Value>]>,
            _retval: Option<&mut Option<V8Value>>,
            exception: Option<&mut CefString>,
        ) -> ::std::os::raw::c_int {
            let payload = arguments
                .and_then(|arguments| arguments.first())
                .and_then(|value| value.as_ref())
                .filter(|value| value.is_string() != 0)
                .map(|value| CefString::from(&value.string_value()).to_string())
                .unwrap_or_default();
            let Some(message) =
                process_message_create(Some(&CefString::from(PAGE_CHROME_MESSAGE_NAME)))
            else {
                let _ = exception;
                return 0;
            };

            let Some(args) = message.argument_list() else {
                let _ = exception;
                return 0;
            };

            args.set_string(0, Some(&CefString::from(payload.as_str())));

            let Some(context) = v8_context_get_current_context() else {
                let _ = exception;
                return 0;
            };

            let Some(frame) = context.frame() else {
                let _ = exception;
                return 0;
            };

            let mut message = message;
            frame.send_process_message(ProcessId::BROWSER, Some(&mut message));
            1
        }
    }
}

impl PageChromeBridgeV8HandlerBuilder {
    fn build() -> V8Handler {
        Self::new(PageChromeBridgeV8Handler)
    }
}

#[derive(Clone)]
struct PageChromeRenderProcessHandler;

wrap_render_process_handler! {
    pub(crate) struct PageChromeRenderProcessHandlerBuilder {
        handler: PageChromeRenderProcessHandler,
    }

    impl RenderProcessHandler {
        fn on_context_created(
            &self,
            _browser: Option<&mut cef::Browser>,
            frame: Option<&mut Frame>,
            context: Option<&mut V8Context>,
        ) {
            let (Some(frame), Some(context)) = (frame, context) else {
                return;
            };

            if frame.is_main() == 0 {
                return;
            }

            let mut handler = PageChromeBridgeV8HandlerBuilder::build();
            let Some(mut bridge) = v8_value_create_function(
                Some(&CefString::from(PAGE_CHROME_BRIDGE_NAME)),
                Some(&mut handler),
            ) else {
                return;
            };

            let Some(global) = context.global() else {
                return;
            };

            global.set_value_bykey(
                Some(&CefString::from(PAGE_CHROME_BRIDGE_NAME)),
                Some(&mut bridge),
                Default::default(),
            );

            let mut result = None;
            let mut eval_exception = None::<cef::V8Exception>;
            if context.eval(
                Some(&CefString::from(PAGE_CHROME_OBSERVER_SCRIPT)),
                Some(&CefString::from("qedit://page_chrome.js")),
                0,
                Some(&mut result),
                Some(&mut eval_exception),
            ) == 0
            {
                if let Some(eval_exception) = eval_exception {
                    log::warn!(
                        "[browser::page_chrome] Failed to install page chrome observer: {}",
                        CefString::from(&eval_exception.message()).to_string()
                    );
                }
            }
        }

        fn on_focused_node_changed(
            &self,
            _browser: Option<&mut cef::Browser>,
            frame: Option<&mut Frame>,
            node: Option<&mut cef::Domnode>,
        ) {
            let Some(frame) = frame else {
                return;
            };

            if frame.is_main() == 0 {
                return;
            }

            let _ = send_text_input_state(frame, node.as_deref());
        }
    }
}

impl PageChromeRenderProcessHandlerBuilder {
    pub(crate) fn build() -> cef::RenderProcessHandler {
        Self::new(PageChromeRenderProcessHandler)
    }
}

fn parse_css_color(value: &str) -> Option<Hsla> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if let Some(components) = value
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_rgb_components(components, false);
    }

    if let Some(components) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_rgb_components(components, true);
    }

    if let Some(components) = value
        .strip_prefix("oklab(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_oklab_components(components);
    }

    if let Some(components) = value
        .strip_prefix("oklch(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_oklch_components(components);
    }

    None
}

fn parse_hex_color(hex: &str) -> Option<Hsla> {
    let expanded = match hex.len() {
        3 => {
            let mut expanded = String::with_capacity(6);
            for ch in hex.chars() {
                expanded.push(ch);
                expanded.push(ch);
            }
            expanded
        }
        6 => hex.to_string(),
        _ => return None,
    };

    let rgb = u32::from_str_radix(&expanded, 16).ok()?;
    let red = ((rgb >> 16) & 0xff) as f32 / 255.0;
    let green = ((rgb >> 8) & 0xff) as f32 / 255.0;
    let blue = (rgb & 0xff) as f32 / 255.0;
    Some(Hsla::from(Rgba {
        r: red,
        g: green,
        b: blue,
        a: 1.0,
    }))
}

fn parse_rgb_components(components: &str, has_alpha: bool) -> Option<Hsla> {
    let parts: Vec<_> = components.split(',').map(str::trim).collect();
    let expected_parts = if has_alpha { 4 } else { 3 };
    if parts.len() != expected_parts {
        return None;
    }

    let red = parse_rgb_channel(parts[0])?;
    let green = parse_rgb_channel(parts[1])?;
    let blue = parse_rgb_channel(parts[2])?;
    let alpha = if has_alpha {
        parse_alpha_channel(parts[3])?
    } else {
        1.0
    };

    if alpha <= 0.0 {
        return None;
    }

    Some(Hsla::from(Rgba {
        r: red / 255.0,
        g: green / 255.0,
        b: blue / 255.0,
        a: alpha,
    }))
}

fn parse_rgb_channel(value: &str) -> Option<f32> {
    let channel = value.parse::<f32>().ok()?;
    if !(0.0..=255.0).contains(&channel) {
        return None;
    }
    Some(channel)
}

fn parse_alpha_channel(value: &str) -> Option<f32> {
    let alpha = value.parse::<f32>().ok()?;
    if !(0.0..=1.0).contains(&alpha) {
        return None;
    }
    Some(alpha)
}

fn parse_oklab_components(components: &str) -> Option<Hsla> {
    let (color_components, alpha) = split_css_color_components(components)?;
    if color_components.len() != 3 {
        return None;
    }

    let lightness = color_components[0].parse::<f32>().ok()?;
    let a = color_components[1].parse::<f32>().ok()?;
    let b = color_components[2].parse::<f32>().ok()?;

    rgba_from_oklab(lightness, a, b, alpha).map(Hsla::from)
}

fn parse_oklch_components(components: &str) -> Option<Hsla> {
    let (color_components, alpha) = split_css_color_components(components)?;
    if color_components.len() != 3 {
        return None;
    }

    let lightness = color_components[0].parse::<f32>().ok()?;
    let chroma = color_components[1].parse::<f32>().ok()?;
    let hue_degrees = color_components[2].parse::<f32>().ok()?;
    let hue_radians = hue_degrees.to_radians();
    let a = chroma * hue_radians.cos();
    let b = chroma * hue_radians.sin();

    rgba_from_oklab(lightness, a, b, alpha).map(Hsla::from)
}

fn split_css_color_components(components: &str) -> Option<(Vec<&str>, f32)> {
    let mut parts = components.split('/');
    let color_components = parts
        .next()?
        .split_whitespace()
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>();
    let alpha = if let Some(alpha) = parts.next() {
        parse_alpha_channel(alpha.trim())?
    } else {
        1.0
    };
    Some((color_components, alpha))
}

fn rgba_from_oklab(lightness: f32, a: f32, b: f32, alpha: f32) -> Option<Rgba> {
    if !(0.0..=1.0).contains(&lightness) || !(0.0..=1.0).contains(&alpha) {
        return None;
    }

    let l = cube(lightness + 0.396_337_78 * a + 0.215_803_76 * b);
    let m = cube(lightness - 0.105_561_346 * a - 0.063_854_17 * b);
    let s = cube(lightness - 0.089_484_18 * a - 1.291_485_5 * b);

    let red_linear = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
    let green_linear = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s;
    let blue_linear = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;

    Some(Rgba {
        r: linear_to_srgb(red_linear),
        g: linear_to_srgb(green_linear),
        b: linear_to_srgb(blue_linear),
        a: alpha,
    })
}

fn cube(value: f32) -> f32 {
    value * value * value
}

fn linear_to_srgb(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.0031308 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::{PageChromeSource, parse_page_chrome_payload};

    #[test]
    fn parse_page_chrome_payload_accepts_hex_color() {
        let chrome =
            parse_page_chrome_payload(r##"{"color":"#1d4e89","source":"sampled_top_edge"}"##)
                .expect("payload should parse");

        assert_eq!(chrome.source, PageChromeSource::SampledTopEdge);
    }

    #[test]
    fn parse_page_chrome_payload_accepts_rgba_color() {
        let chrome = parse_page_chrome_payload(
            r#"{"color":"rgba(29, 78, 137, 0.9)","source":"theme_color_meta"}"#,
        )
        .expect("payload should parse");

        assert_eq!(chrome.source, PageChromeSource::ThemeColorMeta);
    }

    #[test]
    fn parse_page_chrome_payload_rejects_invalid_colors() {
        assert!(
            parse_page_chrome_payload(r#"{"color":"not-a-color","source":"sampled_top_edge"}"#)
                .is_none()
        );
    }

    #[test]
    fn parse_page_chrome_payload_accepts_oklab_color() {
        let chrome = parse_page_chrome_payload(
            r#"{"color":"oklab(0.999994 0.0000455678 0.0000200868 / 0.75)","source":"sampled_top_edge"}"#,
        )
        .expect("payload should parse");

        assert_eq!(chrome.source, PageChromeSource::SampledTopEdge);
    }
}
