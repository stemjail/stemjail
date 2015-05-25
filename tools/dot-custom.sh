#!/bin/sh

# Set domain and access colors

sed -r '
	s/(^ +D_[0-9]+\[[^]]+)(];)$/\1, style="filled", fillcolor="#b3e1ff"\2/;
	s/(^ +A_[0-9]+\[[^]]+)(];)$/\1, style="filled", fillcolor="#97f1ae"\2/;
	/home/ s/( fillcolor=")#[a-f0-9]+/\1#0a9c30/;
	/ &cap; / s/( style=)/ shape=box,\1/;
	'
