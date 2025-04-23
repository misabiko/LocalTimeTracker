<script lang='ts'>
    import { invoke } from '@tauri-apps/api/core';

    let entryMessage = $state('');

    //TODO List entries of the day
	invoke('get_entries', { date: new Date() })
		.then(entries => console.log(entries));

	//TODO Show one entry block
	//TODO Start new entry
	//TODO Update block size of current entry in real time
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

        grid-template-columns: auto 1fr;
        grid-template-rows: repeat(24, 4em);
        display: grid;

		background-color: #1b1b1b;
		padding: 0 0.5em;
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

	.entry-block {
		/*height: 4em;*/
		border-bottom: 1px solid #f0f0f0;
	}
</style>


<div id='timer-topbar'>
	<input type='text' bind:value={entryMessage} placeholder='What are you working on?'/>
</div>
<!--TODO Padding-->
<!--TODO Date title & selector-->
<!--TODO Anchor calendar to bottom with scroll-->
<div id='calendar'>
<!--	TODO Format with Intl-->
	{#each Array(23) as _, i}
		<div class='timestamp'>{i + 1}:00</div>
		<div class='entry-block'></div>
	{/each}
</div>