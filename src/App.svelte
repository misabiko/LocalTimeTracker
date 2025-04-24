<script lang='ts'>
	//TODO Enforce indent to tabs
    import {invoke} from '@tauri-apps/api/core';
    import {onMount} from "svelte";
    import {SvelteDate} from "svelte/reactivity";

    type TimeSheetEntry = {
        description: string
        start_time: SvelteDate
        end_time: SvelteDate | null
        tags: string[]
    }

    //TODO Make deeply readonly
    let entries: Readonly<TimeSheetEntry>[] | null = $state(null);

    let entryMessage = $state('');
    //TODO Get currentEntry from last ongoing entry
	//TODO Warn multiple ongoing entries
	let currentEntryIndex: number | null = $state(null);
    let currentEntry: Readonly<TimeSheetEntry | null> = $derived(currentEntryIndex != null && entries != null ? entries[currentEntryIndex] : null);

    let modalEntryIndex: number | null = $state(null);
    let modalEntry: Readonly<TimeSheetEntry | null> = $derived(modalEntryIndex != null && entries != null ? entries[modalEntryIndex] : null);
    let modalEntryElement: HTMLDialogElement | null = $state(null);

    let currentDate = $state(new Date().toISOString().split('T')[0]);

    $effect(() => {
        invoke('get_entries', { date: currentDate })
            .then(e => {
                entries = (e as TimeSheetEntry[])?.map(e => ({
                    ...e,
					start_time: new SvelteDate(e.start_time),
					end_time: e.end_time ? new SvelteDate(e.end_time) : null,
				})) ?? null;
                console.log($state.snapshot(currentDate), $state.snapshot(entries));
            });
	})

	async function updateEntry(index: number, update: (entry: TimeSheetEntry) => TimeSheetEntry) {
        if (!entries)
            return;

        const entry = entries[index];
        if (!entry)
            return;

        await invoke('update_entry', {
            oldDescription: entry.description,
			oldStartTime: entry.start_time.toISOString(),
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
			startTime: entry.start_time.toISOString(),
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

    let fakeNow = $state(new Date());
    onMount(() => {requestAnimationFrame(updateFakeNow);})

	//TODO Disable realtime out of focus
	function updateFakeNow() {
        const fakeNowRaw = new Date();
        //TODO Clean once we use current date properly
        // fakeNowRaw.setFullYear(fullDate.getFullYear(), fullDate.getMonth(), fullDate.getDate());
        // fakeNowRaw.setHours(fakeNowRaw.getHours() - 6);
        fakeNow = fakeNowRaw;
        requestAnimationFrame(updateFakeNow);
	}

	//TODO Week view

	function getEntryDuration(entry: TimeSheetEntry) {
        const endTime = entry.end_time ?? fakeNow;

        return endTime.getTime() - entry.start_time.getTime();
	}

    function getDecimalHours(date: Date) {
		return date.getHours()
			+ date.getMinutes() / 60
			+ date.getSeconds() / 3600;
	}

    function incrementDate(delta: number) {
        const [year, month, date] = currentDate.split('-').map(n => parseInt(n));
        const newDate = new Date(year, month - 1, date + delta);
		currentDate = newDate.toISOString().split('T')[0];
	}

    function setCurrentDate(newDate: Date) {
        currentDate = newDate.toISOString().split('T')[0];
	}

    const emPerHour = 4;
    const blockMilliToEm = 1 / 1000 / 60 / 60 * emPerHour;

    async function startNewEntry() {
        if (!entries || currentEntry != null)
            return;

        currentEntry = {
			description: entryMessage,
			start_time: new SvelteDate(),
			end_time: null,
			tags: [],
        }

        try {
            const success = await invoke('add_entry', {entry: {
                ...$state.snapshot(currentEntry),
				//TODO Send array to add_entry
				tags: '',
			}});

            if (!success)
                throw new Error('Failed to add entry');

            entries.push(currentEntry);
            entryMessage = '';
        }catch (e) {
            console.error(e);
        }
	}

    async function stopCurrentEntry() {
        if (currentEntryIndex === null)
			return;

		await updateEntry(currentEntryIndex, entry => {
			return {
				...entry,
				end_time: new SvelteDate(),
			}
		});

        currentEntry = null;
	}

	async function setStartDateLocal(index: number, dateTimeLocal: string) {
        if (!entries)
            return;

        const [date, time] = dateTimeLocal.split('T');
        const [year, month, day] = date.split('-').map(n => parseInt(n));
        const [hour, minute] = time.split(':').map(n => parseInt(n));

        return updateEntry(index, entry => {
            const startTime = entry.start_time;
            startTime.setFullYear(year, month - 1, day);
            startTime.setHours(hour, minute);
            entry.start_time.setTime(startTime.getTime());

            return entry;
		})
	}

    async function setEndDateLocal(index: number, dateTimeLocal: string) {
		if (!entries)
			return;
        const [date, time] = dateTimeLocal.split('T');
        const [year, month, day] = date.split('-').map(n => parseInt(n));
        const [hour, minute] = time.split(':').map(n => parseInt(n));

        return updateEntry(index, entry => {
            if (entry.end_time) {
                entry.end_time.setFullYear(year, month - 1, day);
                entry.end_time.setHours(hour, minute);
            }else
                entry.end_time = new SvelteDate(year, month - 1, day, hour, minute);

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
		/*TODO Make dynamic with emPerHour*/
		grid-template-rows: repeat(24, 4em);
		display: grid;

		background-color: #1b1b1b;
		padding: 0 0.5em;

		position: relative;
	}

	/*#time-column {*/
	/*	width: 100px;*/
	/*	flex-shrink: 0;*/
	/*	background-color: #212121;*/
	/*}*/

	.timestamp {
	/*	height: 4em;*/
		padding-top: 3.5em;
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
</style>


<div id='timer-topbar'>
	<input type='text' bind:value={entryMessage} placeholder='What are you working on?'/>
	<button onclick={() => startNewEntry()} disabled={!entryMessage.length}>Start</button>
	<button onclick={() => stopCurrentEntry()} disabled={!currentEntry}>Stop</button>
<!--TODO Manual entry mode-->
</div>
<!--TODO Padding-->
<div id='calendar-controls'>
	<button onclick={() => incrementDate(-1)}>{"<"}</button>
<!--TODO Nice word date for today-->
<!--TODO Custom input with shortcuts integrated-->
	<input id='date' type='date' bind:value={currentDate}/>
	<button onclick={() => incrementDate(1)}>{">"}</button>
	{#if currentDate !== new Date().toISOString().split('T')[0]}
		<button onclick={() => setCurrentDate(new Date())}>Today</button>
	{/if}
<!--TODO Total time-->
</div>
<div id='calendar'>
<!--TODO Format with Intl-->
<!--TODO Trim hours and offset blocks for it-->
<!--TODO Show marker at current time-->
<!--TODO	Start scrolled to marker-->
	{#each Array(23) as _, i}
		<div class='timestamp'>{i + 1}:00</div>
		<div class='entry-row'></div>
	{/each}
	{#if entries !== null}
		{#each entries as entry, i}
			<!--TODO Thicker hitbox on tiny heights-->
			<div
					class='entry-block'
					class:from-toggl={entry.tags.includes('Toggl')}
					style:height={`${getEntryDuration(entry) * blockMilliToEm}em`}
					style:top={`${getDecimalHours(new Date(entry.start_time)) * emPerHour}em`}
					onclick={() => showEntryModal(i)}
					role='button'
			>
				<span>{entry.description}</span>
			</div>
		{/each}
	{/if}
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
			   bind:value={
					() => modalEntry.start_time.toISOString().slice(0, 16),
					v => setStartDateLocal(modalEntry, v)
			   }
		/>
		{#if modalEntry.end_time != null}
			<input type='datetime-local'
				   bind:value={
						() => modalEntry.end_time.toISOString().slice(0, 16),
						v => setEndDateLocal(modalEntry, v)
				   }
			/>
		{/if}
		<input
			type='number'
			readonly={!modalEntry.end_time}
			step={1 / 60}
			bind:value={
				() => {
					const endTime = modalEntry.end_time ?? fakeNow;
					return (endTime.getTime() - modalEntry.start_time.getTime()) / 1000 / 60 / 60;
				},
				v => updateEntry(modalEntryIndex, entry => {
					const newEndTime = new Date(entry.start_time);
					newEndTime.setHours(newEndTime.getHours() + v);
					entry.end_time!.setTime(newEndTime.getTime());
					return entry;
				})
			}
		/>
		<input type='text' readonly value={modalEntry.tags}/>
		<button onclick={() => deleteEntry(modalEntryIndex)}>Delete</button>
	{/if}
	<!--TODO Support closing by clicking on backdrop-->
	<button onclick={() => modalEntryElement?.close()}>Close</button>
</dialog>