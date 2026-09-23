"""Compatibility shim - the atomized visualizer now lives in the visualize/ package.

The 666-line flat module was split into visualize/{context_shape,constants,
results,token_comparison,timeline,compression_efficiency,radar_chart,
summary_dashboard,orchestrator,cli}.py. This shim keeps
`python3 visualize.py <results_dir>` working.
"""

from visualize.cli import main

if __name__ == "__main__":
    main()
