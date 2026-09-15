"use strict";

function hexToHsb(hex) {
	const r = parseInt(hex.slice(1, 3), 16) / 255;
	const g = parseInt(hex.slice(3, 5), 16) / 255;
	const b = parseInt(hex.slice(5, 7), 16) / 255;
	const max = Math.max(r, g, b);
	const min = Math.min(r, g, b);
	const delta = max - min;
	let hue = 0;
	if (delta > 0) {
		if (max === r) hue = ((g - b) / delta) % 6;
		else if (max === g) hue = (b - r) / delta + 2;
		else hue = (r - g) / delta + 4;
		hue = (hue * 60 + 360) % 360;
	}
	const saturation = max === 0 ? 0 : (delta / max) * 100;
	return { hue: Math.round(hue), saturation: Math.round(saturation), brightness: Math.round(max * 100) };
}

function hsbToHex(hue, saturation, brightness) {
	const s = saturation / 100;
	const v = brightness / 100;
	const c = v * s;
	const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
	const m = v - c;
	const sector = Math.floor(hue / 60) % 6;
	const table = [[c, x, 0], [x, c, 0], [0, c, x], [0, x, c], [x, 0, c], [c, 0, x]];
	const [r, g, b] = table[sector];
	const channel = (value) => Math.round((value + m) * 255).toString(16).padStart(2, "0");
	return "#" + channel(r) + channel(g) + channel(b);
}

function bindColorPicker(picker, output, includeBrightness) {
	const describe = (hsb) => "H " + hsb.hue + " S " + hsb.saturation + (includeBrightness ? " B " + hsb.brightness : "");

	document.addEventListener("settings-loaded", () => {
		const hue = PI.settings.hue === undefined ? 30 : PI.settings.hue;
		const saturation = PI.settings.saturation === undefined ? 100 : PI.settings.saturation;
		const brightness = includeBrightness && PI.settings.brightness !== undefined ? PI.settings.brightness : 100;
		picker.value = hsbToHex(hue, saturation, brightness);
		output.textContent = describe({ hue, saturation, brightness });
	});

	picker.addEventListener("input", () => {
		const hsb = hexToHsb(picker.value);
		PI.settings.hue = hsb.hue;
		PI.settings.saturation = hsb.saturation;
		if (includeBrightness) PI.settings.brightness = hsb.brightness;
		output.textContent = describe(hsb);
		PI.save();
	});
}
