import math
from typing import Any
import networkx as nx
from pathlib import Path

from .types import ArtifactKind, Step, BuildCtx


class BuildPlanner:
    """
    Manages a list of ArtifactKinds and Steps to convert between them,
    for the purpose of planning a build process. The internal representation
    is a directed multi-graph, where each node represents an ArtifactKind,
    and each edge represents a Step.

    Each step has a `get_cost` implementation that is provided the build context,
    which allows for determining the optimal (lowest cost) path between two artifact
    kinds (e.g. the user's input program and the final selene simulation binary).
    This allows extensions to add new steps with lower costs to supplant existing
    steps, or to alter the effective 'cost' of their steps based on build context
    (e.g. user-provided choices, or platform-specific steps).

    Each artifact kind has a `matches` method, which is used to determine the
    'kind' of a given resource (e.g. the user's input program). These are checked
    from highest to lowest priority, allowing extensions to supplant or specialise
    existing kinds by adding a new kind with higher priority.
    """

    def __init__(self):
        self.step_graph = nx.MultiDiGraph()

    def add_kind(self, kind: type[ArtifactKind]):
        """
        Add a new artifact kind to the planner. This represents some resource,
        such as a file or variable, that can be processed (via a Step) to produce
        another resource.
        """
        self.step_graph.add_node(kind)

    def identify_kind(self, resource: Any) -> type[ArtifactKind] | None:
        """
        Given some resource (of any type), try to determine the associated
        ArtifactKind using each registered kind's `matches` method, returning
        the highest priority kind that matches the resource.

        If you wish to "override" a kind, e.g. to replace it to add custom steps,
        or to specialise it, you can do so by adding a new kind with higher priority.
        It will then be checked first, and successful matching will prevent checking
        against the original kind.
        """
        for kind in sorted(self.step_graph, key=lambda k: -k.priority):
            if kind.matches(resource):
                return kind
        return None

    def add_step(self, step: type[Step]):
        """
        Add a step to the planner. This step will be used to convert between
        two artifact kinds.

        At the moment, we also only support steps that accept a single artifact
        kind and produce a single output kind. This may change in future.

        Steps have an associated cost, which is used to determine the optimal path
        from some input kind to a required output kind (e.g. the final selene
        simulation binary for the input program). If you wish to override an existing
        build path, you can do so by adding new steps with lower costs. Alternatively,
        you can override the input kind and add custom steps to it.
        """
        if step.input_kind not in self.step_graph:
            raise ValueError(
                f"Input kind '{step.input_kind}' of '{step}' not found in step graph\n"
                f"Available kinds: {list(self.step_graph.nodes)}"
            )
        if step.output_kind not in self.step_graph:
            raise ValueError(
                f"Output kind '{step.output_kind}' of '{step}' not found in step graph\n"
                f"Available kinds: {list(self.step_graph.nodes)}"
            )
        self.step_graph.add_edge(step.input_kind, step.output_kind, step=step)

    def write_dot(self, path: Path, highlighted_steps: list | None = None):
        """
        Write the step graph to a dot file. This can be visualised using Graphviz.
        """
        copy = self.step_graph.copy()
        if highlighted_steps is not None:
            # first gray out all nodes
            for node in copy.nodes:
                copy.nodes[node]["style"] = "filled"
                copy.nodes[node]["color"] = "gray"
            # and all edges
            for kind_from, edges_out in copy.adjacency():
                for kind_to, edges in edges_out.items():
                    for edge in edges.values():
                        edge["style"] = "dashed"
                        edge["color"] = "gray"
            # but highlight the kinds visited along the way
            for step in highlighted_steps:
                copy.nodes[step.input_kind]["style"] = "filled"
                copy.nodes[step.input_kind]["color"] = "black"
                copy.nodes[step.output_kind]["style"] = "filled"
                copy.nodes[step.output_kind]["color"] = "black"
            for kind_from, edges_out in copy.adjacency():
                for kind_to, edges in edges_out.items():
                    for edge in edges.values():
                        if edge["step"] in highlighted_steps:
                            edge["style"] = "solid"
                            edge["color"] = "black"

        nx.drawing.nx_pydot.write_dot(copy, path)

    def _get_reduced_digraph_by_context(self, ctx: BuildCtx) -> nx.DiGraph:
        """
        Return a DiGraph view of the MultiDiGraph where each edge (u, v) is replaced
        by a single edge representing the step of minimal cost given the context.

        Steps of non-finite cost (inf, -inf, nan) are ignored. This is the canonical
        way of ruling out a step given context.

        Steps of finite but negative weight will raise an exception, as it is likely
        that they have a bug in their cost logic.

        Returns:
            A DiGraph with at most one edge per (u, v) from the MultiDiGraph.
        """
        reduced = nx.DiGraph()
        for node in self.step_graph.nodes:
            reduced.add_node(node)
        for kind_from, edges_out in self.step_graph.adjacency():
            for kind_to, edges in edges_out.items():
                lowest_cost_edge = None
                lowest_cost = float("inf")
                for edge in edges.values():
                    step = edge["step"]
                    cost = step.get_cost(ctx)
                    if not math.isfinite(cost):
                        continue
                    if cost < 0:
                        raise ValueError(
                            f"Step {step} has negative cost {cost} for context {ctx}"
                        )
                    if cost < lowest_cost:
                        lowest_cost = cost
                        lowest_cost_edge = step
                if lowest_cost_edge is not None:
                    reduced.add_edge(
                        kind_from, kind_to, step=lowest_cost_edge, cost=lowest_cost
                    )
        return reduced

    def get_optimal_steps_between(
        self,
        input_kind: type[ArtifactKind],
        output_kind: type[ArtifactKind],
        build_ctx: BuildCtx,
    ) -> list[Step]:
        """
        Given an input kind and an output kind, return the optimal path of steps
        between them. This is done by finding the shortest path in the step graph,
        using the cost of each step as the weight of the edge in the graph.

        If no path is found, an exception is raised. This could mean that the input
        and/or output kind has not been registered with this planner, or that they
        are disconnected in the step graph.
        """
        reduced_digraph = self._get_reduced_digraph_by_context(build_ctx)
        if input_kind not in reduced_digraph:
            raise ValueError(f"Input kind '{input_kind}' not found in step graph")
        if output_kind not in reduced_digraph:
            raise ValueError(f"Output kind '{output_kind}' not found in step graph")
        try:
            path = nx.shortest_path(
                reduced_digraph,
                source=input_kind,
                target=output_kind,
                weight=lambda u, v, d: d["cost"],
            )
        except nx.NetworkXNoPath:
            raise ValueError(f"No path found from '{input_kind}' to '{output_kind}'")
        steps = []
        for i in range(len(path) - 1):
            steps.append(reduced_digraph[path[i]][path[i + 1]]["step"])
        return steps
