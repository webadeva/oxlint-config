import { userEvent } from '@storybook/test';

export default { title: 'Context' };

export const B = { play: async (ctx: unknown) => {} };
export const A = { play: async (context) => { await B.play(context); } };
export const C = { play: async ({ canvasElement, ...ctx }) => { await B.play(ctx); } };
export const D = { play: async ({ context }) => { await B.play(context); } };
export const E = { play: async (ctx) => { await B.play({ ...ctx, x: 1 }); } };
export const F = { play: async (ctx) => { await B.play({ x: ctx }); } };
export const G = { play: async function (ctx) { await B.play(ctx); } };
export const H = { play: async () => { await B.play(); } };
export const I = { play: async ({ canvasElement }) => { await B.play(canvasElement); } };
export const J = { play: async (ctx) => { await B.play(ctx, 1); } };
export const K = { play: async (ctx) => { await B.play!(ctx); } };
export const L = { play: async (ctx = {}) => { await B.play(ctx); } };
export const M = { play: async (ctx) => { await B['play'](ctx); } };
export const N = { play: async (ctx) => { await B.play((ctx)); } };
