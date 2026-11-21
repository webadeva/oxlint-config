export default { title: 'Names' };

export const PrimaryButton = { name: 'Primary Button' };
export const Secondary = { storyName: 'Secondary' };
export const Third = { name: 'Third!' };
export const Fourth = () => {};
Fourth.storyName = 'Fourth';
export const ABCDef = { name: 'ABC Def' };
export const story1 = { name: 'Story 1' };
export const snake_case_story = { name: 'Snake Case Story' };
export const WithArgs = { args: {}, name: 'With Args' };
export const Shorthand = { name };
Fourth.storyName = 'Not Fourth';
(Fourth.storyName = 'Fourth');
const x = (Fourth.storyName = 'Fourth');
