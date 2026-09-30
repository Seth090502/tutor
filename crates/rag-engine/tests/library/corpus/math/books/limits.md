# Chapter 3 Limits

This chapter introduces limits, the idea that makes calculus work. A limit describes the value
a function gets close to as its input gets close to some point, whether or not the function is
defined at that point. Every later topic in the course, from derivatives to integrals, is built
on this single notion, so it pays to understand it slowly and with many pictures.

## 3.1 The Idea of a Limit

Imagine walking along the graph of a function toward a chosen input from both sides. If the
heights you pass settle toward one number, that number is the limit. A hole in the graph does
not matter, because the limit only asks about nearby inputs, never the input itself. A classic
example is the ratio of sine x to x near zero: the expression is undefined at zero, yet the
values crowd around one, so the limit equals one. Tables of values and zoomed graphs are good
first tools, but they can mislead when a function oscillates wildly, which is why the careful
epsilon and delta definition exists. That definition says the outputs can be kept within any
tolerance epsilon of the limit by keeping the inputs within some distance delta of the point.

## 3.2 One-Sided Limits

Sometimes a function approaches different heights from the left and from the right. The
left-hand limit looks only at inputs smaller than the point, and the right-hand limit looks only
at inputs larger than it. A step function such as the floor function jumps at every integer, so
its left-hand and right-hand limits disagree there. The two-sided limit exists exactly when both
one-sided limits exist and are equal. Piecewise definitions, absolute values divided by their
argument, and square roots near the edge of their domain are the usual places where one-sided
limits are needed, and sketching the graph near the break point is the quickest check.

## 3.3 Limits at Infinity

A limit at infinity describes the long-run behaviour of a function as the input grows without
bound. For rational functions, compare the degrees of the numerator and denominator: a larger
denominator degree sends the quotient to zero, equal degrees give the ratio of the leading
coefficients, and a larger numerator degree means the quotient grows without bound. Horizontal
asymptotes are the graphical record of finite limits at infinity, while exponential decay,
such as e to the minus x, approaches zero faster than any power of x shrinks.
