import 'jsr:@std/dotenv/load';

const jiraPrefix = Deno.env.get('VITE_JIRA_URL_PREFIX');

const id = Deno.env.get('TEST_JIRA_ID');

const response = await fetch(`${jiraPrefix}rest/api/2/issue/${id}/worklog`,{
	method: 'POST',
	headers: {
		'Authorization': `Basic ${btoa(Deno.env.get('JIRA_USERNAME') + ':' + Deno.env.get('JIRA_PASSWORD'))}`,
		'Content-Type': 'application/json',
		'Accept': 'application/json'
	},
	body: JSON.stringify({
		// started: '2025-05-06T17:36:39.548Z', bad
		started: '2025-05-06T12:34:00.000+0000',// good
		timeSpentSeconds: 15365,
	}),
})
    .then(res => {
		console.log(res);
		return res.json();
	});
console.log(response);