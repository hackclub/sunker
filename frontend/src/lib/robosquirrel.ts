export const INTRO = [
	'Welcome to Sunker, traveller! We need all the help we can get',
	'We are under siege from the alien Dyconids. It is your job to fight them back',
	'That reminds me, I never got your passport'
];

export const TOUR = [
	'Welcome to Headquarters! Start a project with the + in Projects. Ship it to earn resources for the war effort, plus a real prize from Prizes shipped to your door!',
	'Use those resources to start Operations in the map to kick the Dyconids out',
	'However, Operations can be expensive. Collaborate with others just like yourself by coordinating in the Chat. Go ahead and say hi!'
];

export const GUILDS = "One last thing, Sunker is split into different Guilds. While we're all united against the Dyconids, there *is* some friendly competition between the guilds...";

export const FAREWELL = "And that's it for now! Remember, you can always learn more with the help icon in the bottom left. For Sunker!";

export const QUIPS = [
	'Go get those Dyconids!',
	'I have friends everywhere',
	'Watch yourself out there',
	'The best offense is a strong offense',
	'If your enemy is quick to anger, seek to be friendly to him',
	'We must defeat the Dyconids at once',
	'Xₖ = Σₙ₌₀ᴺ⁻¹ xₙ · e^(−2πikn/N)',
	'Go fight for Sunker!'
];

const plain = (line: string) => line.replaceAll('*', '');

export const RATE = Math.max(...[...INTRO, ...TOUR, GUILDS, FAREWELL, ...QUIPS].map((line) => plain(line).length)) / 1000;

export function tilt(previous: number) {
	let next = previous;
	while (Math.abs(next - previous) < 6) next = Math.random() * 30 - 15;
	return next;
}
