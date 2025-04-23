<script lang='ts'>
    import {invoke} from '@tauri-apps/api/core';
    import {onMount} from "svelte";

    let entryMessage = $state('');

    type TimeSheetEntry = {
        description: string
		start_time: string
		end_time: string | null
		tags: string[]
	}

	let entries: TimeSheetEntry[] | null = $state(null);

	invoke('get_entries', { date: '2025-04-02' })
		.then(e => {
            // console.log(entries);
            // entries = (e as TimeSheetEntry[]) ?? null;
            console.log($state.snapshot(entries));
        });

    let currentEntry: TimeSheetEntry = {
		description: 'Test entry',
		start_time: new Date(2025, 4, 2, 10, 30).toISOString(),
		end_time: null,
		tags: []
	};
    let currentEntryDuration = $state(getEntryDuration(currentEntry));
    onMount(() => {
        requestAnimationFrame(updateCurrentEntrySize);
	})

	function updateCurrentEntrySize() {
        currentEntryDuration = getEntryDuration(currentEntry);
        requestAnimationFrame(updateCurrentEntrySize);
	}

	//TODO Start new entry
	//TODO Update block size of current entry in real time
	//TODO Week view

	function getEntryDuration(entry: TimeSheetEntry) {
        const fakeNow = new Date();
        fakeNow.setFullYear(2025, 4, 2);
        fakeNow.setMonth(4, 2);
        fakeNow.setHours(fakeNow.getHours() - 6);
        const endTime = entry.end_time ? new Date(entry.end_time) : fakeNow;

		const start = new Date(entry.start_time);
        return endTime.getTime() - start.getTime();
	}

    function getDecimalHours(date: Date) {
		return date.getHours()
			+ date.getMinutes() / 60
			+ date.getSeconds() / 3600;
	}

    const emPerHour = 4;
    const blockMilliToEm = 1 / 1000 / 60 / 60 * emPerHour;
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
<!--TODO Date title & selector-->
<div id='calendar'>
<!--	TODO Format with Intl-->
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
	<div
			class='entry-block current-entry'
			style:height={`${currentEntryDuration * blockMilliToEm}em`}
			style:top={`${getDecimalHours(new Date(currentEntry.start_time)) * emPerHour}em`}
	>
		<span>{currentEntry.description}</span>
	</div>
</div>