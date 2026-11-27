# Fields

Keys inside `plugin.json`'s `fields[]` array. Curio assigns each field's numeric ID, owner and reader metadata when loading the plugin. See [Register indexable fields](../contributions/fields.md).

## fields[].key {#fields-key}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Unique lowercase machine identifier used in queries, `item.update` and `extractors[].fields` (for example `"model_triangles"` or `"read_state"`).

## fields[].label {#fields-label}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Human-readable display name shown in the Inspector, List view columns, the Filter bar and **Settings › Fields**.

## fields[].type {#fields-type}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge">Required</span></div>
Value storage type: `text`, `integer`, `float`, `boolean`, `date`, `datetime`, `single_select`, `multi_select`, `json`, `url` or `file_ref`.

## fields[].description {#fields-description}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Tooltip and help text displayed in **Settings › Fields**.

## fields[].default_value {#fields-default_value}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Optional default string value for new manual entries.

## fields[].options {#fields-options}
<div class="curio-meta"><span class="curio-badge">string[]</span><span class="curio-badge curio-badge--muted">Default: []</span></div>
Predefined choice values for `single_select` and `multi_select` fields (for example `["unread", "reading", "finished"]`).

## fields[].validation {#fields-validation}
<div class="curio-meta"><span class="curio-badge">object</span><span class="curio-badge curio-badge--muted">Default: {}</span></div>
Validation constraints (`min`, `max`, `max_length`, `pattern`, `required`).

## fields[].format {#fields-format}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: ""</span></div>
Optional display formatting hint.

## fields[].widget {#fields-widget}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: "" ("row")</span></div>
Inspector editor presentation: `"row"` (single line, default when empty), `"block"` (multi-line text box) or `"code"` (monospace multi-line text box). Multi-value `multi_select` fields render as chips automatically.

## fields[].visible {#fields-visible}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: true</span></div>
Whether the field appears in Inspector field widgets and the List view column picker.

## fields[].sort_order {#fields-sort_order}
<div class="curio-meta"><span class="curio-badge">integer</span><span class="curio-badge curio-badge--muted">Default: 0</span></div>
Relative ordering index among fields.

## fields[].own_filter {#fields-own_filter}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: false</span></div>
When `true`, adds the field as a top-level Filter bar option and standalone Smart Folder rule (users can still override this in **Settings › Fields**).

## fields[].filter_kind {#fields-filter_kind}
<div class="curio-meta"><span class="curio-badge">string</span><span class="curio-badge curio-badge--muted">Default: "" (auto)</span></div>
Filter UI style: `"list"`, `"range"`, `"text"`, `"has"` or `""` (selected automatically from `type`).

## fields[].indexed {#fields-indexed}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: true</span></div>
Initial indexing state for file-reading fields. Set `false` for high-cardinality fields (such as `seed` or `model_vertices`) so users can opt in via **Settings › Fields**. Fields without readers are always indexed.

## fields[].pair {#fields-pair}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Key of the primary field this secondary field pairs with (for example `"lora"` on `lora_strength` or `"series"` on `series_index`).

## fields[].filter_group {#fields-filter_group}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Shared label combining multiple numeric fields of this plugin into one multi-range filter popover (for example `"Model size (mm)"`).

## fields[].validation.min {#fields-validation-min}
<div class="curio-meta"><span class="curio-badge">number | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Minimum accepted numeric value for `integer` and `float` fields.

## fields[].validation.max {#fields-validation-max}
<div class="curio-meta"><span class="curio-badge">number | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Maximum accepted numeric value for `integer` and `float` fields.

## fields[].validation.max_length {#fields-validation-max_length}
<div class="curio-meta"><span class="curio-badge">integer | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Maximum character length for text, URL, select, file reference and JSON values.

## fields[].validation.pattern {#fields-validation-pattern}
<div class="curio-meta"><span class="curio-badge">string | null</span><span class="curio-badge curio-badge--muted">Default: null</span></div>
Regular expression that text values must match.

## fields[].validation.required {#fields-validation-required}
<div class="curio-meta"><span class="curio-badge">boolean</span><span class="curio-badge curio-badge--muted">Default: false</span></div>
When `true`, clearing or submitting an empty value is rejected.
