<script lang='ts'>
	//TODO Enforce indent to tabs
    import {invoke} from '@tauri-apps/api/core';
    import {onMount} from "svelte";
    import { Temporal } from '@js-temporal/polyfill';

    type TimeSheetEntry = {
        description: string
        start_time: number
        end_time: number | null
		//TODO Port back to array
        tags: string
    }

    //TODO Make deeply readonly
    let entries: Readonly<TimeSheetEntry>[] | null = $state(null);

    let entryMessage = $state('');
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

    $effect(() => {
        invoke('get_entries', { date: currentDate.toString() })
            .then(e => {
                entries = (e as TimeSheetEntry[]) ?? null;
                console.debug($state.snapshot(currentDate), $state.snapshot(entries));
            });
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
			description: entryMessage,
			start_time: new Date().getTime(),
			end_time: null,
			tags: '',
        };

        try {
            const success = await invoke('add_entry', {entry});

            if (!success)
                throw new Error('Failed to add entry');

            entries.push(entry);
            currentEntryIndex = entries.length - 1;
            entryMessage = '';
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
		padding: 1.5em;
		margin-bottom: 1em;
	}

	#timer-topbar > input {
		width: 100%;
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
</style>

<div id='timer-topbar'>
<!--TODO Suggest previous descriptions, and copy tags-->
<!--TODO Todoist style tags entry-->
	<input type='text' bind:value={entryMessage} placeholder='What are you working on?'/>
	<button onclick={() => startNewEntry()} disabled={!entryMessage.length}>Start</button>
	<button onclick={() => stopCurrentEntry()} disabled={currentEntryIndex == null}>Stop</button>
<!--TODO Manual entry mode-->
</div>
<!--TODO Padding-->
<div id='calendar-controls'>
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
	<span>Total Hours: {totalHoursTodayStr}</span>
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
				() => getEntryDurationHours(modalEntry),
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
		{#if modalEntry.end_time == null}
			<button onclick={() => updateEntry(modalEntryIndex, entry => {
				entry.end_time = new Date().getTime();
				return entry;
			})}>Stop</button>
		{/if}
		<button onclick={() => deleteEntry(modalEntryIndex)}>Delete</button>
	{/if}
	<!--TODO Support closing by clicking on backdrop-->
	<button onclick={() => modalEntryElement?.close()}>Close</button>
</dialog>