﻿<script lang='ts'>
	//TODO Enforce indent to tabs
	import {invoke} from '@tauri-apps/api/core';
	import {onMount} from "svelte";
	import { Temporal } from '@js-temporal/polyfill';
    import {open} from '@tauri-apps/plugin-shell';

	type TimeSheetEntry = {
		description: string
		start_time: number
		end_time: number | null
		//TODO +2 Port back to array
		tags: string
		properties: Record<string, string>
	}
	type TimeSheetEntryTemplate = Omit<TimeSheetEntry, 'start_time' | 'end_time'>;

	//TODO Make deeply readonly
	let entries: Readonly<TimeSheetEntry>[] | null = $state(null);

	let inputEntry: TimeSheetEntryTemplate = $state({
		description: '',
		tags: '',
		properties: {},
	});
	//TODO Get currentEntry from last ongoing entry
	//TODO Warn multiple ongoing entries
	let currentEntryIndex: number | null = $state(null);
	const currentEntry: Readonly<TimeSheetEntry | null> = $derived(currentEntryIndex != null && entries != null ? entries[currentEntryIndex] : null);

	let modalEntryIndex: number | null = $state(null);
	let modalEntry: Readonly<TimeSheetEntry | null> = $derived(modalEntryIndex != null && entries != null ? entries[modalEntryIndex] : null);
	let modalEntryElement: HTMLDialogElement | null = $state(null);

	//TODO Persist changes
	let currentDate = $state(Temporal.Now.plainDateISO());
	let firstViewHour = $state(6);
	let lastViewHour = $state(20);

	let totalHoursToday = $derived.by(() => {
		if (entries === null)
			return 0;

		//Assuming entries only has today's
		return entries.reduce((total, entry) => {
			return total + getEntryDurationHours(entry);
		}, 0);
	});
	const durationFormat = new Intl.DurationFormat('en-CA', { style: 'short' });
	let totalHoursTodayStr = $derived(durationFormat.format({
		hours: Math.floor(totalHoursToday),
		minutes: Math.floor((totalHoursToday % 1) * 60),
	}));
	let endTimeStr = $derived(Intl.DateTimeFormat('en-CA', {
		hour: '2-digit',
		minute: '2-digit',
	}).format(Temporal.Now.instant().add({seconds: Math.round((8 - totalHoursToday) * 60 * 60)}).epochMilliseconds));

	let entrySuggestions: TimeSheetEntryTemplate[] = $state([]);
	let inputFocused = $state(false);
	let enableSuggestions = $state(false);

	$effect(() => {
		invoke<TimeSheetEntry[]>('get_date_entries', { date: currentDate.toString() })
			.then(e => {
				entries = e as TimeSheetEntry[];
				console.debug($state.snapshot(currentDate), $state.snapshot(entries));
			});
	})

	$effect(() => {
		if (!inputEntry.description) {
			entrySuggestions = [];
			return;
		}

		//TODO Add cooldown for suggestions
		if (enableSuggestions) {
			invoke<TimeSheetEntryTemplate[]>('suggest_entry_descriptions', {partial: inputEntry.description})
				.then(suggestions => {
					if (Array.isArray(suggestions.tags))
						suggestions.tags = suggestions.tags.join(',');
					entrySuggestions = suggestions;
				});
		}
	})

	async function updateEntry(index: number, update: (entry: TimeSheetEntry) => TimeSheetEntry) {
		if (!entries)
			return;

		const entry = entries[index];
		if (!entry) {
			console.warn('No entry found at index', index);
			return;
		}

		await invoke('update_entry', {
			oldDescription: entry.description,
			oldStartTime: entry.start_time,
			entry: update(entry),
		});

		entries[index] = entry;
	}

	async function copyAndStartEntry(index: number) {
		if (!entries) {
			console.warn("entries is null");
			return;
		}
		const copiedEntry = entries[index];
		if (!copiedEntry) {
			console.warn("No entry found at index", index);
			return;
		}

		//TODO temp, should handle multi currentEntry, but no way to distinguish/list them yet
		if (currentEntryIndex != null) {
			console.warn("Already have an entry");
			// return;
			currentEntryIndex = null;
		}

		const newEntry: TimeSheetEntry = {
			...copiedEntry,
			start_time: new Date().getTime(),
			end_time: null,
		};

		//TODO Handle template properties vs entry properties
		delete newEntry.properties.jira_worklog_id;

		try {
			const success = await invoke('add_entry', {entry: newEntry});

			if (!success)
				throw new Error('Failed to add entry');

			entries.push(newEntry);
			currentEntryIndex = entries.length - 1;
			inputEntry.description = '';
		}catch (e) {
			console.error(e);
		}

	}

	async function deleteEntry(index: number) {
		if (!entries)
			return;
		const entry = entries[index];
		const success = await invoke('delete_entry', {
			description: entry.description,
			startTime: entry.start_time,
		});
		if (!success)
			throw new Error('Failed to delete entry');
		entries?.splice(index, 1);
		if (currentEntryIndex === index)
			currentEntryIndex = null;
		if (modalEntryIndex === index) {
			modalEntryIndex = null;
			modalEntryElement?.close();
		}
	}

	let fakeNow: number = $state(new Date().getTime());
	let currentTimeMarker: HTMLDivElement | null = $state(null);
	onMount(() => {
		requestAnimationFrame(updateFakeNow);

		if (currentTimeMarker === null)
			console.warn('No current time marker found');
		else
			currentTimeMarker.scrollIntoView({block: 'center'});
	})

	//TODO Disable realtime out of focus
	function updateFakeNow() {
		fakeNow = new Date().getTime();
		requestAnimationFrame(updateFakeNow);
	}

	//TODO Week view

	function getEntryDurationMilli(entry: TimeSheetEntry): number {
		const endTime = entry.end_time ?? fakeNow;

		return endTime - entry.start_time;
	}

	function getEntryDurationHours(entry: TimeSheetEntry): number {
		const endTime = entry.end_time ?? fakeNow;
		const value = (endTime - entry.start_time) / 1000 / 60 / 60;
		return value;
	}

	function getDecimalHours(epochMs: number): number {
		const plainDate = Temporal.Instant.fromEpochMilliseconds(epochMs)
			.toZonedDateTimeISO(Temporal.Now.timeZoneId()).startOfDay();
		const ms = epochMs - plainDate.epochMilliseconds;
		return ms / 1000 / 60 / 60;
	}

	let emPerHour = $state(4);
	let blockMilliToEm = $derived(1 / 1000 / 60 / 60 * emPerHour);

	async function startNewEntry() {
		if (!entries) {
			console.warn("entries is null");
			return;
		}

		if (currentEntryIndex != null) {
			console.warn("Already have an entry");
			// return;
			currentEntryIndex = null;
		}

		const entry: TimeSheetEntry = {
			...inputEntry,
			start_time: new Date().getTime(),
			end_time: null,
		};

		try {
			const success = await invoke('add_entry', {entry});

			if (!success)
				throw new Error('Failed to add entry');

			entries.push(entry);
			currentEntryIndex = entries.length - 1;
			inputEntry.description = '';
			inputEntry.tags = '';
			inputEntry.properties = {};
		}catch (e) {
			console.error(e);
		}
	}

	async function stopCurrentEntry() {
		if (currentEntryIndex === null) {
			console.warn("No current entry to stop");
			return;
		}

		await updateEntry(currentEntryIndex, entry => {
			return {
				...entry,
				end_time: new Date().getTime(),
			}
		});

		currentEntryIndex = null;
	}

	async function setStartDateLocal(index: number, dateTimeLocal: string) {
		if (!entries)
			return;

		const dt = Temporal.PlainDateTime.from(dateTimeLocal);
		const timeZone = Temporal.Now.timeZoneId();

		await updateEntry(index, entry => {
			const newTime = dt.toZonedDateTime(timeZone).epochMilliseconds;
			entry.start_time = newTime;

			return entry;
		})
	}

	async function setEndDateLocal(index: number, dateTimeLocal: string) {
		if (!entries)
			return;
		const dt = Temporal.PlainDateTime.from(dateTimeLocal);
		const timeZone = Temporal.Now.timeZoneId();
		const ms = dt.toZonedDateTime(timeZone).epochMilliseconds;

		await updateEntry(index, entry => {
			entry.end_time = ms;

			return entry;
		});
	}

	function showEntryModal(index: number) {
		if (modalEntryIndex !== null)
			return;
		//Could replace with 'Import' tag or field
		if (entries === null || entries[index].tags.includes('Toggl'))
			return;
		modalEntryIndex = index;
		modalEntryElement?.showModal();
	}

	function onEntryModalClose() {
		modalEntryIndex = null;
	}

	function epochMsToDateTimeLocal(epochMs: number): string {
		const zoned = Temporal.Instant.fromEpochMilliseconds(epochMs)
			.toZonedDateTimeISO(Temporal.Now.timeZoneId());
		const plainDT = zoned.toPlainDateTime();
		return plainDT.toString();
	}

	function getEntryBlockTop(entry: TimeSheetEntry): string {
		const hours = getDecimalHours(entry.start_time) - firstViewHour;
		return `${hours * emPerHour}em`;
	}

	function selectSuggestion(s: TimeSheetEntryTemplate) {
		if (Array.isArray(s.tags))
			s.tags = s.tags.join(',');
		inputEntry = s;
		entrySuggestions = [];
	}

	let holiday_count = $state(0);
	let week_remaining_hours: number | null = $state(null);
	async function updateRemainingWeekHours() {
		week_remaining_hours = await invoke<number>('get_remaining_week_hours', {holidays: $state.snapshot(holiday_count)})
	}
</script>

<style>
	:global(body) {
		/*TODO Store colors in variables*/
		background-color: #313131;
		color: #f0f0f0;
		font-family: sans-serif;
		display: flex;
		flex-direction: column;
		align-items: stretch;
		margin: 0;
		height: 100vh;
	}

	#timer-topbar {
        padding: 1.5em 1.5em 0;
        margin-bottom: 1em;
	}

	#entry-input {
		position: relative;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
	}

	#calendar {
		/*TODO Minimalist scrollbar*/
		overflow-y: scroll;
		flex-shrink: 1;

		grid-template-columns: 5em 1fr;
		display: grid;

		background-color: #1b1b1b;
		padding: 0 0.5em;

		position: relative;
	}

	.timestamp {
		vertical-align: bottom;
		margin-right: 4px;
		text-align: right;
	}

	.entry-row {
		/*height: 4em;*/
		border-bottom: 1px solid #f0f0f0;
	}

	/*TODO Handle overlapping blocks by offsetting to the side*/
	/*TODO Have pointer cursor on block*/
	.entry-block {
		/*TODO Try dynamic color via tags and project*/
		background-color: red;
		position: absolute;
		width: calc(100% - 5em - 16px);
		opacity: .5;
		margin-left: calc(5em + 8px);
		border-radius: 4px;
	}

	/*TODO temp*/
	.entry-block.from-toggl {
		background-color: mediumpurple;
	}

	.entry-block > span {
		padding: 0 0.5em;
	}

	#entry-modal {
		background-color: #555555;
		color: #f0f0f0;
		border: none;
		padding: 1em;
		width: 300px;
		/*TODO Center vertically*/
		top: 100px;
	}

	#current-time-marker {
		position: absolute;
		width: calc(100% - 5em - 16px);
		margin-left: calc(5em + 8px);
		height: 1px;
		background-color: #0064ff;
	}

	dialog[open] {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
	}

	#timer-topbar ul {
		position: absolute;
		top: 4.3em;
		left: 0;
		right: 0;
		z-index: 10;
		background: #222;
		margin: 0;
		padding: 0;
		list-style: none;
		border-radius: 0 0 4px 4px;
		box-shadow: 0 2px 8px #0008;
	}
	#timer-topbar ul li {
		padding: 0.5em;
		cursor: pointer;
	}

	#time-info {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
	}
</style>

<div id='timer-topbar'>
	<!--TODO Todoist style tags entry-->
	<div id='entry-input'>
		<input type='text' bind:value={inputEntry.description} placeholder='What are you working on?'
			onfocus={() => inputFocused = true}
			onblur={() => inputFocused = false}
		/>

		<!--TODO +1 Add inputs for tags and properties-->
		<input type='text' bind:value={inputEntry.tags}/>
		{#if inputEntry.properties.jira}
			<input type='text' bind:value={inputEntry.properties.jira}/>
		{/if}

		{#if inputFocused && entrySuggestions.length}
			<ul>
				{#each entrySuggestions as suggestion}
					<!--TODO Add tags-->
					<!--TODO Make anchor or button-->
					<li onmousedown={() => selectSuggestion(suggestion)}>{suggestion.description}</li>
				{/each}
			</ul>
		{/if}
		<label>
			Enable Suggestions
			<input type='checkbox' bind:checked={enableSuggestions} title='Enable Suggestions'/>
		</label>
	</div>
	<button onclick={() => startNewEntry()} disabled={!inputEntry.description.length}>Start</button>
	<button onclick={() => stopCurrentEntry()} disabled={currentEntryIndex == null}>Stop</button>
	<!--TODO +1 List ongoing timers-->
	<!--TODO +1 Mark one timer as "current timer" to switch from-->
</div>
<!--TODO Padding-->
<div id='calendar-controls'>
	<div>
		<button onclick={() => currentDate = currentDate.add({days: -1})}>{"<"}</button>
		<!--TODO Nice word date for today-->
		<!--TODO Custom input with shortcuts integrated-->
		<input id='date' type='date' value={currentDate} onchange={e => {
			currentDate = Temporal.PlainDate.from(e.target.value);
		}}/>
		<button onclick={() => currentDate = currentDate.add({days: 1})}>{">"}</button>
		{#if !currentDate.equals(Temporal.Now.plainDateISO())}
			<button onclick={() => currentDate = Temporal.Now.plainDateISO()}>Today</button>
		{/if}
		<input type='number' step='1' min='0' max={lastViewHour} bind:value={firstViewHour}/>
		<input type='number' step='1' min={firstViewHour + 1} max='23' bind:value={lastViewHour}/>
		<input type='number' step='0.1' bind:value={emPerHour} title='Em Per Hour'/>
	</div>
	<div id='time-info'>
		<span>Total Hours: {totalHoursTodayStr}</span>
		{#if currentDate.equals(Temporal.Now.plainDateISO())}
			<span>End Time: {endTimeStr}</span>
		{/if}
		<span>Remaining Week Hours: {week_remaining_hours?.toFixed(2)}</span>
		<div>
			<input type='number' bind:value={holiday_count} min='0' max='4'/>
			<button onclick={() => updateRemainingWeekHours()}>Update</button>
		</div>
	</div>
</div>
<div id='calendar' style:grid-template-rows={`repeat(${lastViewHour - firstViewHour + 1}, ${emPerHour}em)`}>
	{#each Array(lastViewHour - firstViewHour) as _, i}
		<div class='timestamp' style:padding-top={`${emPerHour - 0.5}em`}>
			{#if i + 1 + firstViewHour > 12}
				{(i + 1 + firstViewHour) - 12}:00 PM
			{:else}
				{i + 1 + firstViewHour}:00 AM
			{/if}
		</div>
		<div class='entry-row'></div>
	{/each}
	{#if entries !== null}
		{#each entries as entry, i}
			<div
					class='entry-block'
					class:from-toggl={entry.tags.includes('Toggl')}
					style:height={`${getEntryDurationMilli(entry) * blockMilliToEm}em`}
					style:top={getEntryBlockTop(entry)}
					onclick={() => showEntryModal(i)}
					role='button'
			>
				<span>{entry.description}</span>
			</div>
		{/each}
	{/if}
	<div
		id='current-time-marker'
		style:top={`${(getDecimalHours(fakeNow) - firstViewHour) * emPerHour}em`}
		bind:this={currentTimeMarker}
	></div>
</div>

<dialog id='entry-modal' bind:this={modalEntryElement} onclose={() => onEntryModalClose()}>
	{#if modalEntryIndex != null && modalEntry != null}
		<input
			type='text'
			value={modalEntry.description}
			onchange={e => updateEntry(modalEntryIndex, entry => {
				entry.description = (e.target as HTMLInputElement).value;
				return entry;
			})}
		/>
		<input type='datetime-local'
			   value={epochMsToDateTimeLocal(modalEntry.start_time)}
			   oninput={e => setStartDateLocal(modalEntryIndex, e.target.value)}
		/>
		{#if modalEntry.end_time != null}
			<input type='datetime-local'
				value={epochMsToDateTimeLocal(modalEntry.end_time)}
				oninput={e => setEndDateLocal(modalEntryIndex, e.target.value)}
			/>
		{/if}
		<input
			type='number'
			readonly={!modalEntry.end_time}
			step='0.01'
			bind:value={
				() => getEntryDurationHours(modalEntry).toFixed(3),
				v => updateEntry(modalEntryIndex, entry => {
					const newEndTime = Temporal.Instant
						.fromEpochMilliseconds(entry.start_time)
						.add({nanoseconds: Math.floor(v * 3600e9)});
					entry.end_time = newEndTime.epochMilliseconds;
					return entry;
				})
			}
		/>
		<input
				type='text'
				value={modalEntry.tags}
				onchange={e => updateEntry(modalEntryIndex, entry => {
					// entry.tags = (e.target as HTMLInputElement).value.split(',');
					entry.tags = (e.target as HTMLInputElement).value;
					return entry;
				})}
		/>

		<label>
			Jira
			<input type='text' value={modalEntry.properties.jira} onchange={e => updateEntry(modalEntryIndex, entry => {
				entry.properties.jira = (e.target as HTMLInputElement).value;
				return entry;
			})}/>
		</label>
		{#if modalEntry.properties.jira}
			{#if modalEntry.properties.jira_worklog_id}
				{@const worklogURL = `${import.meta.env.VITE_JIRA_URL_PREFIX}browse/${modalEntry.properties.jira}?focusedId=${modalEntry.properties.jira_worklog_id}#worklog-${modalEntry.properties.jira_worklog_id}`}
				<a href={worklogURL} onclick={async (e) => {
					e.preventDefault();
					await open(worklogURL);
				}}>{modalEntry.properties.jira}</a>
			{:else}
				{@const jiraURL = `${import.meta.env.VITE_JIRA_URL_PREFIX}browse/${modalEntry.properties.jira}`}
				<a href={jiraURL} onclick={async (e) => {
					e.preventDefault();
					await open(jiraURL);
				}}>{modalEntry.properties.jira}</a>
			{/if}
		{/if}
		<!--TODO Add properties from UI-->
		<!--TODO Delete properties from UI-->
		{#each Object.entries(modalEntry.properties) as [key, value] (key)}
			{#if key !== 'jira'}
				<label>
					{key}
					<input type='text' value={value} onchange={e => updateEntry(modalEntryIndex, entry => {
						entry.properties[key] = (e.target as HTMLInputElement).value;
						return entry;
					})}/>
				</label>
			{/if}
		{/each}

		{#if modalEntry.end_time == null}
			<button onclick={() => updateEntry(modalEntryIndex, entry => {
				entry.end_time = new Date().getTime();
				return entry;
			})}>Stop</button>
		{:else}
			<button onclick={() => updateEntry(modalEntryIndex, entry => {
				entry.end_time = null;
				return entry;
			})}>Continue</button>
		{/if}
		<button onclick={() => copyAndStartEntry(modalEntryIndex)}>Copy & Start</button>
		<button onclick={() => deleteEntry(modalEntryIndex)}>Delete</button>
	{/if}
	<!--TODO Support closing by clicking on backdrop-->
	<button onclick={() => modalEntryElement?.close()}>Close</button>
</dialog>