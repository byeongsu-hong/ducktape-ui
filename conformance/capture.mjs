import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { chromium } from "playwright";

const referenceDir = process.env.DUCKTAPE_DESIGN_SYSTEM;
if (!referenceDir) {
  throw new Error("set DUCKTAPE_DESIGN_SYSTEM to the approved React reference");
}

const root = resolve(referenceDir);
const outputDir = resolve("expected/web");
const url = "http://127.0.0.1:4178";
const server = spawn("npm", ["run", "dev", "--", "--host", "127.0.0.1", "--port", "4178"], {
  cwd: root,
  stdio: "inherit",
});

try {
  await waitForServer(url);
  await mkdir(outputDir, { recursive: true });
  const browser = await chromium.launch({
    headless: true,
    executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE,
  });
  try {
    const page = await browser.newPage({
      viewport: { width: 1440, height: 900 },
      deviceScaleFactor: 2,
    });
    await page.goto(url, { waitUntil: "networkidle" });
    await page.evaluate(() => document.fonts.ready);

    const cases = await captureCases(page, outputDir);
    const contract = {
      version: 1,
      source: {
        package: "ducktape-design-system-react",
        version: JSON.parse(await readFile(join(root, "package.json"), "utf8")).version,
        sha256: await treeHash(root),
        viewport: [1440, 900],
        deviceScaleFactor: 2,
      },
      tolerances: {
        geometryPx: 1,
        colorChannel: 2,
        changedPixelRatio: 0.20,
        changedPixelChannel: 32,
      },
      cases,
    };
    await writeFile(resolve("expected/reference.json"), `${JSON.stringify(contract, null, 2)}\n`);
  } finally {
    await browser.close();
  }
} finally {
  server.kill("SIGTERM");
}

async function captureCases(page, directory) {
  const stateMatrix = page.getByText("STATE MATRIX · 같은 버튼의 5가지 상태", { exact: true });
  const inputHeading = page.getByText("INPUT · 포커스는 1.5px 잉크 테두리", { exact: true });
  const cardsHeading = page.getByText("CARDS · PANELS", { exact: true });
  const primary = page.getByRole("button", { name: "Send invite →", exact: true }).first();
  const focusedInput = inputHeading.locator("xpath=following-sibling::div[1]");
  const placeholderInput = inputHeading.locator("xpath=following-sibling::div[2]");
  const hover = await buttonStateClone(page, primary, stateMatrix.locator("xpath=following-sibling::div[1]/*[2]/*[2]"), "hover");
  const disabled = await buttonStateClone(page, primary, stateMatrix.locator("xpath=following-sibling::div[1]/*[4]/*[2]"), "disabled");
  const targets = [
    ["typography.display", page.getByText("Welcome to Ducktape", { exact: true }).first(), {
      foreground: "primary",
    }],
    ["typography.machine", page.getByText("127.0.0.1:8844 · height 84,912", { exact: true }), {
      foreground: "secondary_foreground",
    }],
    ["button.default", primary, {
      background: "primary", foreground: "primary_foreground",
    }],
    ["button.hover", hover, {
      background: "primary_hover", foreground: "primary_foreground",
    }],
    ["button.disabled", disabled, {
      background: "disabled", foreground: "disabled_foreground",
    }],
    ["button.secondary", page.getByRole("button", { name: "Cancel", exact: true }), {
      background: "secondary", foreground: "secondary_foreground", border: "control_line",
    }],
    ["button.outline", page.getByRole("button", { name: "Propose", exact: true }), {
      background: "card", foreground: "accent_foreground", border: "border",
    }],
    ["input.focused", focusedInput, {
      background: "card", border: "ring",
    }, focusedInput.locator("span").nth(1)],
    ["input.placeholder", placeholderInput, {
      background: "card", border: "border",
    }, placeholderInput.locator("span")],
    ["card.proposal", cardsHeading.locator("xpath=following-sibling::div[1]"), {
      background: "brand_background", border: "brand_line",
    }],
  ];

  const result = {};
  for (const [id, locator, roles, textLocator = locator] of targets) {
    await locator.scrollIntoViewIfNeeded();
    const box = await locator.boundingBox();
    if (!box) throw new Error(`no bounding box for ${id}`);
    const surfaceStyle = await locator.evaluate((element) => {
      const computed = getComputedStyle(element);
      let background = computed.backgroundColor;
      for (let ancestor = element.parentElement; background === "rgba(0, 0, 0, 0)" && ancestor; ancestor = ancestor.parentElement) {
        background = getComputedStyle(ancestor).backgroundColor;
      }
      return {
        background,
        borderColor: computed.borderColor,
        borderWidth: parseFloat(computed.borderTopWidth),
        radius: parseFloat(computed.borderTopLeftRadius),
        padding: [
          parseFloat(computed.paddingTop),
          parseFloat(computed.paddingRight),
          parseFloat(computed.paddingBottom),
          parseFloat(computed.paddingLeft),
        ],
      };
    });
    const textStyle = await textLocator.evaluate((element) => {
      const computed = getComputedStyle(element);
      return {
        foreground: computed.color,
        fontFamily: computed.fontFamily,
        fontSize: parseFloat(computed.fontSize),
        fontWeight: Number(computed.fontWeight),
        lineHeight: computed.lineHeight === "normal" ? null : parseFloat(computed.lineHeight),
      };
    });
    const file = `${id.replaceAll(".", "-")}.png`;
    await locator.screenshot({ path: join(directory, file), animations: "disabled" });
    result[id] = {
      size: [round(box.width), round(box.height)],
      roles,
      style: { ...surfaceStyle, ...textStyle },
      screenshot: `web/${file}`,
    };
  }
  return result;
}

async function buttonStateClone(page, source, state, id) {
  const colors = await state.evaluate((element) => {
    const style = getComputedStyle(element);
    return { background: style.backgroundColor, foreground: style.color };
  });
  await source.evaluate((element, { colors, id }) => {
    const clone = element.cloneNode(true);
    clone.dataset.conformance = id;
    clone.style.background = colors.background;
    clone.style.color = colors.foreground;
    clone.style.position = "absolute";
    clone.style.left = "30px";
    clone.style.top = `${document.documentElement.scrollHeight + 30}px`;
    document.body.append(clone);
  }, { colors, id });
  return page.locator(`[data-conformance="${id}"]`);
}

async function waitForServer(target) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try {
      const response = await fetch(target);
      if (response.ok) return;
    } catch {}
    await new Promise((done) => setTimeout(done, 100));
  }
  throw new Error(`reference server did not start at ${target}`);
}

async function treeHash(directory) {
  const hash = createHash("sha256");
  for (const file of await sourceFiles(directory)) {
    hash.update(file.slice(directory.length + 1));
    hash.update(await readFile(file));
  }
  return hash.digest("hex");
}

async function sourceFiles(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (["node_modules", "dist", ".git"].includes(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await sourceFiles(path));
    else if (!["preview.html"].includes(basename(path))) files.push(path);
  }
  return files.sort();
}

function round(value) {
  return Math.round(value * 1000) / 1000;
}
