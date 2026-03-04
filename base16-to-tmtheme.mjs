#!/usr/bin/env node
// One-shot converter: base16/Gogh YAML → tmTheme (for syntect)
// Reads from ~/.local/share/themes/, writes .tmTheme files to output dir.

import { readFileSync, writeFileSync, readdirSync, mkdirSync } from "fs";
import { join, basename, extname } from "path";

const THEMES_DIR =
  process.argv[2] || join(process.env.HOME, ".local/share/themes");
const OUT_DIR = process.argv[3] || join(process.cwd(), "tmthemes");

// --- Minimal YAML parser (handles both flat and nested palette formats) ---

function parseYaml(text) {
  const result = {};
  let currentSection = null;

  for (const line of text.split("\n")) {
    if (!line.trim() || line.trim() === "---") continue;

    // Extract key and value, respecting quotes
    const indent = line.match(/^(\s*)/)[1].length;
    const kvMatch = line.match(/^\s*([A-Za-z_][\w]*)\s*:\s*(.*)/);
    if (!kvMatch) continue;

    const key = kvMatch[1];
    let rawVal = kvMatch[2].trim();

    // Extract quoted value, or bare value (strip trailing comment)
    let value;
    const quoted = rawVal.match(/^(['"])(.*?)\1/);
    if (quoted) {
      value = quoted[2];
    } else if (rawVal === "") {
      // Section header (e.g. "palette:")
      value = null;
    } else {
      // Bare value — strip trailing # comment
      value = rawVal.replace(/\s+#\s.*$/, "").trim();
    }

    if (value === null) {
      currentSection = key;
      result[currentSection] = {};
      continue;
    }

    if (indent >= 2 && currentSection) {
      result[currentSection][key] = value;
    } else {
      currentSection = null;
      result[key] = value;
    }
  }
  return result;
}

// --- Color math (Lab-space blending, matching base16changer) ---

function hexToRgb(hex) {
  hex = hex.replace(/^#/, "");
  return [
    parseInt(hex.slice(0, 2), 16) / 255,
    parseInt(hex.slice(2, 4), 16) / 255,
    parseInt(hex.slice(4, 6), 16) / 255,
  ];
}

function rgbToHex([r, g, b]) {
  const c = (v) =>
    Math.round(Math.max(0, Math.min(1, v)) * 255)
      .toString(16)
      .padStart(2, "0");
  return c(r) + c(g) + c(b);
}

function linearize(c) {
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

function delinearize(c) {
  return c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
}

function rgbToLab([r, g, b]) {
  r = linearize(r);
  g = linearize(g);
  b = linearize(b);
  let x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047;
  let y = 0.2126729 * r + 0.7151522 * g + 0.072175 * b;
  let z = (0.0193339 * r + 0.119192 * g + 0.9503041 * b) / 1.08883;
  const f = (t) => (t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116);
  x = f(x);
  y = f(y);
  z = f(z);
  return [116 * y - 16, 500 * (x - y), 200 * (y - z)];
}

function labToRgb([L, a, b]) {
  let y = (L + 16) / 116;
  let x = a / 500 + y;
  let z = y - b / 200;
  const finv = (t) =>
    t > 0.206893 ? t * t * t : (t - 16 / 116) / 7.787;
  x = finv(x) * 0.95047;
  y = finv(y);
  z = finv(z) * 1.08883;
  let r = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
  let g = -0.969266 * x + 1.8760108 * y + 0.041556 * z;
  let bl = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;
  return [delinearize(r), delinearize(g), delinearize(bl)];
}

function blendLab(hex1, hex2, t) {
  const lab1 = rgbToLab(hexToRgb(hex1));
  const lab2 = rgbToLab(hexToRgb(hex2));
  const blended = lab1.map((v, i) => v + t * (lab2[i] - v));
  return rgbToHex(labToRgb(blended));
}

// --- Normalize color: strip #, lowercase ---

function norm(c) {
  return (c || "").replace(/^#/, "").toLowerCase();
}

// --- Format detection & conversion to base16 palette ---

function toBase16(parsed) {
  // Base16 format: has palette.base00
  if (parsed.palette && parsed.palette.base00) {
    const p = {};
    for (const [k, v] of Object.entries(parsed.palette)) {
      p[k] = norm(v);
    }
    return {
      name: parsed.name || "Unknown",
      author: parsed.author || "",
      variant: parsed.variant || "dark",
      ...p,
    };
  }

  // Gogh format: has color_01
  if (parsed.color_01) {
    const bg = norm(parsed.background);
    const fg = norm(parsed.foreground);
    const red = norm(parsed.color_02);
    const yellow = norm(parsed.color_04);
    const orange = blendLab(red, yellow, 0.5);
    const brown = blendLab(orange, bg, 0.4);

    return {
      name: parsed.name || "Unknown",
      author: parsed.author || "",
      variant: parsed.variant || "dark",
      base00: bg,
      base01: blendLab(bg, fg, 0.1),
      base02: blendLab(bg, fg, 0.2),
      base03: norm(parsed.color_09),
      base04: blendLab(bg, fg, 0.4),
      base05: fg,
      base06: blendLab(bg, fg, 0.8),
      base07: norm(parsed.color_16),
      base08: red,
      base09: orange,
      base0A: yellow,
      base0B: norm(parsed.color_03),
      base0C: norm(parsed.color_07),
      base0D: norm(parsed.color_05),
      base0E: norm(parsed.color_06),
      base0F: brown,
    };
  }

  return null;
}

// --- tmTheme XML generation ---

function scope(name, scopes, fg, extra = "") {
  return `		<dict>
			<key>name</key>
			<string>${name}</string>
			<key>scope</key>
			<string>${scopes}</string>
			<key>settings</key>
			<dict>
				<key>foreground</key>
				<string>#${fg}</string>${extra}
			</dict>
		</dict>`;
}

function fontStyle(style) {
  return `
				<key>fontStyle</key>
				<string>${style}</string>`;
}

function bgFg(bg, fg) {
  return `
				<key>background</key>
				<string>#${bg}</string>
				<key>foreground</key>
				<string>#${fg}</string>`;
}

function toTmTheme(b) {
  const variant = b.variant || "dark";
  const slug = b.name.toLowerCase().replace(/[^a-z0-9]+/g, "-");

  // base16 standard mapping to TextMate scopes
  const scopes = [
    scope("Comments", "comment, punctuation.definition.comment", b.base03),
    scope("Punctuation", "punctuation.definition.string, punctuation.definition.variable, punctuation.definition.parameters, punctuation.definition.array", b.base05),
    scope("Operators", "keyword.operator", b.base05),
    scope("Keywords", "keyword, storage", b.base0E),
    scope("Variables", "variable, variable.other", b.base08),
    scope("Functions", "entity.name.function, meta.require, support.function.any-method, variable.function", b.base0D),
    scope("Classes", "support.class, entity.name.class, entity.name.type.class, entity.name.type", b.base0A),
    scope("Methods", "keyword.other.special-method", b.base0D),
    scope("Support", "support.function", b.base0C),
    scope("Strings", "string, constant.other.symbol, entity.other.inherited-class", b.base0B),
    scope("Numbers", "constant.numeric", b.base09),
    scope("Constants", "constant, constant.language", b.base09),
    scope("Tags", "entity.name.tag", b.base08),
    scope("Attributes", "entity.other.attribute-name", b.base09),
    scope("Attribute IDs", "entity.other.attribute-name.id, punctuation.definition.entity", b.base0D),
    scope("Selector", "meta.selector", b.base0E),
    scope("Headings", "markup.heading punctuation.definition.heading, entity.name.section", b.base0D, fontStyle("bold")),
    scope("Bold", "markup.bold, punctuation.definition.bold", b.base0A, fontStyle("bold")),
    scope("Italic", "markup.italic, punctuation.definition.italic", b.base0E, fontStyle("italic")),
    scope("Code", "markup.raw.inline", b.base0B),
    scope("Link Text", "string.other.link", b.base0D),
    scope("Link URL", "meta.link", b.base0C),
    scope("Lists", "markup.list", b.base08),
    scope("Quotes", "markup.quote", b.base0A),
    scope("Separator", "meta.separator", b.base05),
    scope("Inserted", "markup.inserted, markup.inserted.git_gutter", b.base0B),
    scope("Deleted", "markup.deleted, markup.deleted.git_gutter", b.base08),
    scope("Changed", "markup.changed, markup.changed.git_gutter", b.base0E),
    scope("Regular Expressions", "string.regexp", b.base0C),
    scope("Escape Characters", "constant.character.escape", b.base0C),
    scope("Embedded", "punctuation.section.embedded, variable.interpolation", b.base0F),
    scope("Invalid", "invalid.illegal", b.base00, `${bgFg(b.base08, b.base00)}`),
  ];

  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>name</key>
	<string>${b.name}</string>
	<key>author</key>
	<string>${b.author}</string>
	<key>colorSpaceName</key>
	<string>sRGB</string>
	<key>semanticClass</key>
	<string>theme.${variant}.${slug}</string>
	<key>settings</key>
	<array>
		<dict>
			<key>settings</key>
			<dict>
				<key>background</key>
				<string>#${b.base00}</string>
				<key>foreground</key>
				<string>#${b.base05}</string>
				<key>caret</key>
				<string>#${b.base05}</string>
				<key>selection</key>
				<string>#${b.base02}</string>
				<key>invisibles</key>
				<string>#${b.base03}</string>
				<key>lineHighlight</key>
				<string>#${b.base01}</string>
				<key>guide</key>
				<string>#${b.base02}</string>
				<key>activeGuide</key>
				<string>#${b.base04}</string>
				<key>stackGuide</key>
				<string>#${b.base01}</string>
			</dict>
		</dict>
${scopes.join("\n")}
	</array>
	<key>uuid</key>
	<string>${crypto.randomUUID()}</string>
</dict>
</plist>
`;
}

// --- Main ---

mkdirSync(OUT_DIR, { recursive: true });

const files = readdirSync(THEMES_DIR).filter(
  (f) => f.endsWith(".yaml") || f.endsWith(".yml")
);

let converted = 0;
let skipped = 0;

for (const file of files) {
  const path = join(THEMES_DIR, file);
  try {
    const parsed = parseYaml(readFileSync(path, "utf8"));
    const b16 = toBase16(parsed);
    if (!b16) {
      console.error(`SKIP (unrecognized format): ${file}`);
      skipped++;
      continue;
    }
    const outName = basename(file, extname(file)) + ".tmTheme";
    writeFileSync(join(OUT_DIR, outName), toTmTheme(b16));
    console.log(`OK: ${file} → ${outName}`);
    converted++;
  } catch (e) {
    console.error(`ERR: ${file}: ${e.message}`);
    skipped++;
  }
}

console.log(`\nDone: ${converted} converted, ${skipped} skipped → ${OUT_DIR}`);
