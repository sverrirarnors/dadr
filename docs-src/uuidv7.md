# UUID v7 generator

Every DADR record has a UUID v7 in its front-matter `id` field. Its first 12 hex
characters are the record's short id, the form to link it by, as in
`[[01938b8e4d70]]`. The filename is up to you; a short kebab-case slug of the
title, such as `income-preprocessing.md`, is suggested.

You may use this page when you are drafting DADR records by hand and need a fresh identifier.


<button id="uuidv7-generate" class="md-button md-button--primary" type="button">Generate UUID v7</button>

<div class="uuidv7-field">
  <label for="uuidv7-current">Full UUIDv7</label>
  <div class="uuidv7-copy-row">
    <input id="uuidv7-current" class="uuidv7-output" type="text" readonly spellcheck="false">
    <button id="uuidv7-copy" class="md-button" type="button">Copy UUID</button>
  </div>
</div>

<div class="uuidv7-field">
  <label for="uuidv7-short">Short id, for links</label>
  <div class="uuidv7-copy-row">
    <input id="uuidv7-short" class="uuidv7-output" type="text" readonly spellcheck="false">
    <button id="uuidv7-copy-short" class="md-button" type="button">Copy short id</button>
  </div>
</div>

<p id="uuidv7-status" class="uuidv7-status" role="status"></p>

This generator runs entirely in your browser. It writes the current Unix millisecond timestamp into the UUIDv7 timestamp field and fills the remaining random bits with `crypto.getRandomValues()`.

<style>
  .uuidv7-field {
    margin-bottom: 0.75rem;
  }

  .uuidv7-tool label {
    color: var(--md-default-fg-color--light);
    display: block;
    font-size: 0.7rem;
    font-weight: 700;
    margin-bottom: 0.25rem;
  }

  .uuidv7-copy-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .uuidv7-output {
    background: var(--md-code-bg-color);
    border: 1px solid var(--md-default-fg-color--lightest);
    border-radius: 4px;
    color: var(--md-code-fg-color);
    flex: 1 1 20rem;
    font: 0.85rem var(--md-code-font-family);
    min-width: 0;
    padding: 0.45rem 0.55rem;
  }

  .uuidv7-status {
    color: var(--md-default-fg-color--light);
    margin: 0.75rem 0 0;
    min-height: 1.2em;
  }

  @media screen and (max-width: 30em) {
    .uuidv7-tool {
      padding: 0.75rem;
    }

    .uuidv7-copy-row .md-button {
      flex: 1 1 100%;
      text-align: center;
    }
  }
</style>

<script>
(() => {
  const currentInput = document.getElementById("uuidv7-current");
  const generateButton = document.getElementById("uuidv7-generate");
  const copyButton = document.getElementById("uuidv7-copy");
  const shortOutput = document.getElementById("uuidv7-short");
  const copyShortButton = document.getElementById("uuidv7-copy-short");
  const statusOutput = document.getElementById("uuidv7-status");

  const hasCrypto = globalThis.crypto && typeof globalThis.crypto.getRandomValues === "function";

  function setStatus(message) {
    statusOutput.textContent = message;
  }

  function bytesToUuid(bytes) {
    const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
    return [
      hex.slice(0, 8),
      hex.slice(8, 12),
      hex.slice(12, 16),
      hex.slice(16, 20),
      hex.slice(20),
    ].join("-");
  }

  function generateUuidV7() {
    if (!hasCrypto) {
      throw new Error("This browser does not expose crypto.getRandomValues().");
    }

    const bytes = new Uint8Array(16);
    globalThis.crypto.getRandomValues(bytes);

    const millis = BigInt(Date.now());
    bytes[0] = Number((millis >> 40n) & 0xffn);
    bytes[1] = Number((millis >> 32n) & 0xffn);
    bytes[2] = Number((millis >> 24n) & 0xffn);
    bytes[3] = Number((millis >> 16n) & 0xffn);
    bytes[4] = Number((millis >> 8n) & 0xffn);
    bytes[5] = Number(millis & 0xffn);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    return bytesToUuid(bytes);
  }

  function renderUuid(uuid) {
    currentInput.value = uuid;
    shortOutput.value = uuid.replaceAll("-", "").slice(0, 12);
  }

  async function copyText(text, successMessage) {
    if (!text) {
      setStatus("Nothing to copy.");
      return;
    }

    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
    } else {
      const textArea = document.createElement("textarea");
      textArea.value = text;
      textArea.setAttribute("readonly", "");
      textArea.style.position = "fixed";
      textArea.style.left = "-9999px";
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand("copy");
      document.body.removeChild(textArea);
    }

    setStatus(successMessage);
  }

  function generateOne() {
    try {
      const uuid = generateUuidV7();
      renderUuid(uuid);
      setStatus("Generated UUIDv7.");
    } catch (error) {
      setStatus(error.message);
    }
  }

  generateButton.addEventListener("click", generateOne);
  copyButton.addEventListener("click", () => copyText(currentInput.value, "Copied UUID."));
  copyShortButton.addEventListener("click", () => copyText(shortOutput.value, "Copied short id."));

  generateOne();
})();
</script>
