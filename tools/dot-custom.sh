#!/bin/sh

# Set domain and access colors

sed -r '
	s/(^ +D_[0-9]+\[[^]]+)(];)$/\1, style="filled", fillcolor="#64c3ff"\2/;
	s/(^ +A_[0-9]+\[[^]]+)(];)$/\1, style="filled", fillcolor="#97f1ae", shape=box\2/;
	/home/ s/( fillcolor=")#[a-f0-9]+/\1#00e73c/;
	/ &cap; / s/( fillcolor=")#[a-f0-9]+/\1#b3e1ff/;
	'
