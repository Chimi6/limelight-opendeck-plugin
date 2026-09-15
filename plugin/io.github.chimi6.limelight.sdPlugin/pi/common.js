"use strict";

const PI = {
	socket: null,
	context: null,
	action: null,
	settings: {},
	targets: null,

	send(payload) {
		this.socket.send(JSON.stringify({ event: "sendToPlugin", action: this.action, context: this.context, payload }));
	},

	save() {
		this.settings.v = 1;
		this.socket.send(JSON.stringify({ event: "setSettings", context: this.context, payload: this.settings }));
	},

	identify(id) {
		this.send({ event: "identify", id });
	},
};

function connectElgatoStreamDeckSocket(port, uuid, registerEvent, info, actionInfo) {
	const action = JSON.parse(actionInfo);
	PI.context = action.context;
	PI.action = action.action;
	PI.settings = (action.payload && action.payload.settings) || {};

	PI.socket = new WebSocket("ws://localhost:" + port);
	PI.socket.onopen = () => {
		PI.socket.send(JSON.stringify({ event: registerEvent, uuid }));
		PI.send({ event: "getTargets" });
	};
	PI.socket.onmessage = (message) => {
		const data = JSON.parse(message.data);
		if (data.event === "sendToPropertyInspector" && data.payload && data.payload.event === "targets") {
			PI.targets = data.payload;
			renderTargets();
			renderBanner();
		}
		if (data.event === "didReceiveSettings") {
			PI.settings = (data.payload && data.payload.settings) || {};
			loadFields();
			renderTargets();
		}
	};

	bindFields();
	loadFields();
	renderTargets();
}

function fieldElements() {
	return Array.from(document.querySelectorAll("[data-setting]"));
}

function fieldType(el) {
	if (el.type === "checkbox") return "checkbox";
	if (el.type === "range" || el.type === "number") return "number";
	return "text";
}

function readField(el) {
	const type = fieldType(el);
	if (type === "checkbox") return el.checked;
	if (type === "number") return el.value === "" ? undefined : Number(el.value);
	return el.value;
}

function defaultFor(el) {
	const fallback = el.dataset.default;
	if (fieldType(el) === "checkbox") return fallback === "true";
	return fallback === undefined ? "" : fallback;
}

function updateOutput(el) {
	const output = document.querySelector('output[data-for="' + el.dataset.setting + '"]');
	if (output) output.textContent = el.value + (el.dataset.suffix || "");
}

function bindFields() {
	for (const el of fieldElements()) {
		const eventName = el.type === "range" ? "input" : "change";
		el.addEventListener(eventName, () => {
			const value = readField(el);
			if (value === undefined || value === "") delete PI.settings[el.dataset.setting];
			else PI.settings[el.dataset.setting] = value;
			updateOutput(el);
			PI.save();
		});
	}
}

function loadFields() {
	for (const el of fieldElements()) {
		let value = PI.settings[el.dataset.setting];
		if (value === undefined) value = defaultFor(el);
		if (fieldType(el) === "checkbox") el.checked = Boolean(value);
		else el.value = value;
		updateOutput(el);
	}
	document.dispatchEvent(new CustomEvent("settings-loaded"));
}

function targetValue(target) {
	if (!target || !target.kind) return "";
	if (target.kind === "all") return "all";
	return target.id ? target.kind + ":" + target.id : "";
}

function renderTargets() {
	const select = document.getElementById("target");
	if (!select) return;
	const filter = document.body.dataset.targets || "any";
	const current = PI.settings.target || {};
	const currentValue = targetValue(current);
	const lights = PI.targets ? PI.targets.lights : [];
	const groups = PI.targets ? PI.targets.groups : [];

	select.innerHTML = "";
	const add = (value, label, disabled) => {
		const option = document.createElement("option");
		option.value = value;
		option.textContent = label;
		option.disabled = Boolean(disabled);
		select.appendChild(option);
	};

	add("", "Choose a target", true);
	if (filter === "any") add("all", "All lights");
	for (const light of lights) {
		if (filter === "color" && !light.color_capable) continue;
		add("light:" + light.id, light.name + (light.reachable ? "" : " (offline)"));
	}
	if (filter === "any") {
		for (const group of groups) add("group:" + group.name, group.name + " (group)");
	}
	const known = Array.from(select.options).some((option) => option.value === currentValue);
	if (currentValue && !known) add(currentValue, (current.id || "All lights") + " (missing)");
	select.value = currentValue;

	select.onchange = () => {
		const value = select.value;
		if (value === "all") {
			PI.settings.target = { kind: "all", id: "" };
		} else {
			const [kind, ...rest] = value.split(":");
			PI.settings.target = { kind, id: rest.join(":") };
		}
		PI.save();
		document.dispatchEvent(new CustomEvent("target-changed"));
	};

	const identifyButton = document.getElementById("identify");
	if (identifyButton) {
		identifyButton.hidden = current.kind !== "light";
		identifyButton.onclick = () => PI.identify(current.id);
	}

	const empty = document.getElementById("no-lights");
	if (empty) empty.hidden = !(PI.targets && PI.targets.daemon.online && lights.length === 0);
}

function renderBanner() {
	let banner = document.getElementById("daemon-banner");
	if (!banner) {
		banner = document.createElement("div");
		banner.id = "daemon-banner";
		banner.className = "banner";
		document.body.prepend(banner);
	}
	const online = PI.targets && PI.targets.daemon && PI.targets.daemon.online;
	banner.hidden = Boolean(online);
	if (online) return;

	banner.innerHTML = "<span>The LimeLight daemon (keylightd) is not running.</span><button id=\"start-daemon\">Start daemon</button>";
	const button = document.getElementById("start-daemon");
	button.onclick = () => {
		button.disabled = true;
		button.textContent = "Starting";
		PI.send({ event: "startDaemon" });
	};
}
