<script lang='ts'>
    import {invoke} from '@tauri-apps/api/core';
    import {onMount} from "svelte";

    //TODO Figure out an id thing to modify existing entries (need to resolve diff start date, and can't rely just on description)
    let entryMessage = $state('');

    type TimeSheetEntry = {
        description: string
		start_time: string
		end_time: string | null
		tags: string[]
	}

	let entries: TimeSheetEntry[] | null = $state(null);

    let currentDate = $state(new Date().toISOString().split('T')[0]);
    let fullDate = $derived.by(() => {
		const [year, month, date] = currentDate.split('-').map(n => parseInt(n));
        return new Date(year, month - 1, date);
    });

    $effect(() => {
        invoke('get_entries', { date: currentDate })
            .then(e => {
                entries = (e as TimeSheetEntry[]) ?? null;
                console.log($state.snapshot(currentDate), $state.snapshot(entries));
            });
	})


    // let currentEntry: TimeSheetEntry = {
	// 	description: 'Test entry',
	// 	start_time: new Date(2025, 4, 2, 10, 30).toISOString(),
	// 	end_time: null,
	// 	tags: []
	// };
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

	//TODO Start new entry
	//TODO Week view

	function getEntryDuration(entry: TimeSheetEntry) {
        const endTime = entry.end_time ? new Date(entry.end_time) : fakeNow;

		const start = new Date(entry.start_time);
        return endTime.getTime() - start.getTime();
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

    async function addTestEntry() {
        const entry = {
			description: entryMessage,
			start_time: new Date().toISOString(),
			end_time: new Date(new Date().getTime() + 1000 * 60 * 60).toISOString(),
			//TODO Handle string vs array for deserializing
			tags: 'test',
		};

        console.log(entry);

        try {
            const success = await invoke('add_entry', {entry});

            if (!success)
                throw new Error('Failed to add entry');
            entries = [...(entries ?? []), entry];
            entryMessage = '';
        }catch (e) {
			console.error(e);
		}
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

	.entry-block {
		background-color: red;
		position: absolute;
		width: calc(100% - 5em - 16px);
		opacity: .5;
		margin-left: calc(5em + 8px);
		border-radius: 4px;
	}

	.entry-block > span {
		padding: 0 0.5em;
	}
</style>


<div id='timer-topbar'>
	<input type='text' bind:value={entryMessage} placeholder='What are you working on?'/>
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
	<button onclick={() => addTestEntry()} disabled={!entryMessage.length}>Add Entry</button>
</div>
<div id='calendar'>
<!--	TODO Format with Intl-->
<!--	TODO Trim hours and offset blocks for it-->
	{#each Array(23) as _, i}
		<div class='timestamp'>{i + 1}:00</div>
		<div class='entry-row'></div>
	{/each}
	{#if entries !== null}
		{#each entries as entry}
			<div
					class='entry-block'
					style:height={`${getEntryDuration(entry) * blockMilliToEm}em`}
					style:top={`${getDecimalHours(new Date(entry.start_time)) * emPerHour}em`}
			>
				<span>{entry.description}</span>
			</div>
		{/each}
	{/if}
<!--	<div-->
<!--			class='entry-block current-entry'-->
<!--			style:height={`${getEntryDuration(currentEntry) * blockMilliToEm}em`}-->
<!--			style:top={`${getDecimalHours(new Date(currentEntry.start_time)) * emPerHour}em`}-->
<!--	>-->
<!--		<span>{currentEntry.description}</span>-->
<!--	</div>-->
</div>