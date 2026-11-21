export default { title: 'Reexport', excludeStories: ['Local'] };

const Local = {};
export { Local };
export { Remote } from './remote';
