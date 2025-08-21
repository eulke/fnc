use crate::config::types::Route;
use crate::error::{HttpDiffError, Result};
use std::collections::{HashMap, HashSet, VecDeque};

/// Represents an execution batch containing routes that can be executed concurrently
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionBatch {
    /// Routes in this batch that can be executed concurrently
    pub routes: Vec<String>,
    /// Batch number (0 = first batch, 1 = second batch, etc.)
    pub batch_number: usize,
}

impl ExecutionBatch {
    /// Create a new execution batch
    pub fn new(batch_number: usize) -> Self {
        Self {
            routes: Vec::new(),
            batch_number,
        }
    }

    /// Add a route to this batch
    pub fn add_route(&mut self, route_name: String) {
        self.routes.push(route_name);
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// Get the number of routes in this batch
    pub fn len(&self) -> usize {
        self.routes.len()
    }
}

/// Represents a complete execution plan with ordered batches
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    /// Ordered batches of routes for execution
    pub batches: Vec<ExecutionBatch>,
    /// Total number of routes in the plan
    pub total_routes: usize,
}

impl ExecutionPlan {
    /// Create a new empty execution plan
    pub fn new() -> Self {
        Self {
            batches: Vec::new(),
            total_routes: 0,
        }
    }

    /// Add a batch to the execution plan
    pub fn add_batch(&mut self, batch: ExecutionBatch) {
        self.total_routes += batch.len();
        self.batches.push(batch);
    }

    /// Check if the execution plan is empty
    pub fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }

    /// Get the total number of batches
    pub fn batch_count(&self) -> usize {
        self.batches.len()
    }

    /// Get all route names in execution order
    pub fn get_all_routes(&self) -> Vec<String> {
        let mut routes = Vec::new();
        for batch in &self.batches {
            routes.extend(batch.routes.clone());
        }
        routes
    }

    /// Get routes that can be executed in the first batch (no dependencies)
    pub fn get_first_batch_routes(&self) -> Option<&ExecutionBatch> {
        self.batches.first()
    }

    /// Validate that all routes are accounted for
    pub fn validate_completeness(&self, expected_routes: &HashSet<String>) -> Result<()> {
        let plan_routes: HashSet<String> = self
            .batches
            .iter()
            .flat_map(|batch| batch.routes.iter().cloned())
            .collect();

        if plan_routes.len() != expected_routes.len() {
            return Err(HttpDiffError::invalid_config(format!(
                "Execution plan contains {} routes but expected {}",
                plan_routes.len(),
                expected_routes.len()
            )));
        }

        for expected_route in expected_routes {
            if !plan_routes.contains(expected_route) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Route '{}' is missing from execution plan",
                    expected_route
                )));
            }
        }

        Ok(())
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Directed graph structure for representing route dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Adjacency list: route_name -> set of routes that depend on it
    dependencies: HashMap<String, HashSet<String>>,
    /// Reverse adjacency list: route_name -> set of routes it depends on
    dependents: HashMap<String, HashSet<String>>,
    /// Set of all route names in the graph
    routes: HashSet<String>,
    /// Cached root routes for performance
    root_routes_cache: std::cell::RefCell<Option<Vec<String>>>,
    /// Cached topological ordering
    topo_order_cache: std::cell::RefCell<Option<Vec<String>>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            routes: HashSet::new(),
            root_routes_cache: std::cell::RefCell::new(None),
            topo_order_cache: std::cell::RefCell::new(None),
        }
    }

    /// Add a route to the graph
    pub fn add_route(&mut self, route_name: String) {
        self.routes.insert(route_name.clone());
        self.dependencies.entry(route_name.clone()).or_insert_with(HashSet::new);
        self.dependents.entry(route_name).or_insert_with(HashSet::new);
        self.invalidate_caches();
    }

    /// Add a dependency edge: dependent_route depends on dependency_route
    pub fn add_dependency(&mut self, dependent_route: String, dependency_route: String) -> Result<()> {
        // Validate that both routes exist
        if !self.routes.contains(&dependent_route) {
            return Err(HttpDiffError::invalid_config(format!(
                "Cannot add dependency: route '{}' does not exist in the graph",
                dependent_route
            )));
        }
        if !self.routes.contains(&dependency_route) {
            return Err(HttpDiffError::invalid_config(format!(
                "Cannot add dependency: route '{}' does not exist in the graph",
                dependency_route
            )));
        }

        // Add to dependencies: dependency_route -> dependent_route
        self.dependencies
            .entry(dependency_route.clone())
            .or_insert_with(HashSet::new)
            .insert(dependent_route.clone());

        // Add to dependents: dependent_route -> dependency_route  
        self.dependents
            .entry(dependent_route)
            .or_insert_with(HashSet::new)
            .insert(dependency_route);

        self.invalidate_caches();
        Ok(())
    }

    /// Get all routes that depend on the given route
    pub fn get_dependents(&self, route_name: &str) -> HashSet<String> {
        self.dependencies
            .get(route_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all routes that the given route depends on
    pub fn get_dependencies(&self, route_name: &str) -> HashSet<String> {
        self.dependents
            .get(route_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all route names in the graph
    pub fn get_all_routes(&self) -> &HashSet<String> {
        &self.routes
    }

    /// Get routes that have no dependencies (can be executed first)
    pub fn get_root_routes(&self) -> Vec<String> {
        // Check cache first
        if let Some(cached) = self.root_routes_cache.borrow().as_ref() {
            return cached.clone();
        }

        // Compute root routes
        let root_routes: Vec<String> = self.routes
            .iter()
            .filter(|route| self.get_dependencies(route).is_empty())
            .cloned()
            .collect();

        // Cache the result
        *self.root_routes_cache.borrow_mut() = Some(root_routes.clone());

        root_routes
    }

    /// Check if the graph contains a specific route
    pub fn contains_route(&self, route_name: &str) -> bool {
        self.routes.contains(route_name)
    }

    /// Get the total number of routes in the graph
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Detect circular dependencies using DFS cycle detection
    pub fn detect_circular_dependencies(&self) -> Result<()> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for route in &self.routes {
            if !visited.contains(route) {
                if let Err(cycle) = self.dfs_cycle_detection(route, &mut visited, &mut recursion_stack) {
                    return Err(cycle);
                }
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection
    fn dfs_cycle_detection(
        &self,
        route: &str,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> Result<()> {
        visited.insert(route.to_string());
        recursion_stack.insert(route.to_string());

        // Visit all routes that this route depends on
        for dependency in &self.get_dependencies(route) {
            if !visited.contains(dependency) {
                if let Err(e) = self.dfs_cycle_detection(dependency, visited, recursion_stack) {
                    return Err(e);
                }
            } else if recursion_stack.contains(dependency) {
                // Found a back edge - circular dependency detected
                return Err(HttpDiffError::invalid_config(format!(
                    "Circular dependency detected: route '{}' depends on '{}' which creates a cycle",
                    route, dependency
                )));
            }
        }

        recursion_stack.remove(route);
        Ok(())
    }

    /// Invalidate all performance caches when the graph structure changes
    fn invalidate_caches(&self) {
        *self.root_routes_cache.borrow_mut() = None;
        *self.topo_order_cache.borrow_mut() = None;
    }

    /// Get a cached topological ordering if available
    pub fn get_topological_order(&self) -> Result<Vec<String>> {
        // Check cache first
        if let Some(cached) = self.topo_order_cache.borrow().as_ref() {
            return Ok(cached.clone());
        }

        // Compute topological ordering using Kahn's algorithm
        let mut in_degree = HashMap::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Initialize in-degree count
        for route in &self.routes {
            in_degree.insert(route.clone(), self.get_dependencies(route).len());
        }

        // Find all nodes with no incoming edges
        for (route, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(route.clone());
            }
        }

        // Process nodes in topological order
        while let Some(route) = queue.pop_front() {
            result.push(route.clone());

            // Reduce in-degree for all dependents
            for dependent in &self.get_dependents(&route) {
                if let Some(degree) = in_degree.get_mut(dependent) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }

        // Check if we processed all nodes (no cycles)
        if result.len() != self.routes.len() {
            return Err(HttpDiffError::invalid_config(
                "Cannot compute topological order: circular dependency detected".to_string(),
            ));
        }

        // Cache the result
        *self.topo_order_cache.borrow_mut() = Some(result.clone());

        Ok(result)
    }

    /// Perform a batch operation to add multiple routes and dependencies efficiently
    pub fn batch_update<F>(&mut self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        // Temporarily disable cache invalidation
        let old_dependencies = self.dependencies.clone();
        let old_dependents = self.dependents.clone();
        let old_routes = self.routes.clone();

        match update_fn(self) {
            Ok(()) => {
                // Only invalidate caches once after all updates
                self.invalidate_caches();
                Ok(())
            }
            Err(e) => {
                // Restore previous state on error
                self.dependencies = old_dependencies;
                self.dependents = old_dependents;
                self.routes = old_routes;
                self.invalidate_caches();
                Err(e)
            }
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolver for managing route dependencies and computing execution order
#[derive(Debug)]
pub struct DependencyResolver {
    graph: DependencyGraph,
}

impl DependencyResolver {
    /// Create a new dependency resolver from a list of routes
    pub fn from_routes(routes: &[Route]) -> Result<Self> {
        let mut graph = DependencyGraph::new();

        // First pass: add all routes to the graph
        for route in routes {
            graph.add_route(route.name.clone());
        }

        // Second pass: add dependencies
        for route in routes {
            if let Some(dependencies) = &route.depends_on {
                for dependency in dependencies {
                    // Validate that the dependency route exists
                    if !graph.contains_route(dependency) {
                        return Err(HttpDiffError::invalid_config(format!(
                            "Route '{}' depends on non-existent route '{}'",
                            route.name, dependency
                        )));
                    }
                    graph.add_dependency(route.name.clone(), dependency.clone())?;
                }
            }
        }

        // Validate no circular dependencies
        graph.detect_circular_dependencies()?;

        Ok(Self { graph })
    }

    /// Create a new dependency resolver from an existing graph
    pub fn from_graph(graph: DependencyGraph) -> Result<Self> {
        // Validate no circular dependencies
        graph.detect_circular_dependencies()?;
        Ok(Self { graph })
    }

    /// Get the underlying dependency graph (read-only access)
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Compute the execution plan using topological sorting
    pub fn compute_execution_plan(&self) -> Result<ExecutionPlan> {
        self.compute_execution_plan_filtered(&self.graph.routes.clone())
    }

    /// Compute the execution plan for a specific subset of routes
    pub fn compute_execution_plan_filtered(&self, route_filter: &HashSet<String>) -> Result<ExecutionPlan> {
        // Validate that all filtered routes exist in the graph
        for route in route_filter {
            if !self.graph.contains_route(route) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Filtered route '{}' does not exist in dependency graph",
                    route
                )));
            }
        }

        // Find all required dependencies for the filtered routes
        let required_routes = self.compute_required_dependencies(route_filter)?;
        
        let mut plan = ExecutionPlan::new();
        let mut in_degree = HashMap::new();
        let mut remaining_routes = required_routes.clone();

        // Calculate in-degree for each route (number of dependencies within the required set)
        for route in &required_routes {
            let dependencies = self.graph.get_dependencies(route);
            let required_dependency_count = dependencies
                .iter()
                .filter(|dep| required_routes.contains(*dep))
                .count();
            in_degree.insert(route.clone(), required_dependency_count);
        }

        let mut batch_number = 0;

        // Continue until all routes are processed
        while !remaining_routes.is_empty() {
            let mut current_batch = ExecutionBatch::new(batch_number);

            // Find all routes with in-degree 0 (no remaining dependencies)
            let ready_routes: Vec<String> = remaining_routes
                .iter()
                .filter(|route| in_degree[*route] == 0)
                .cloned()
                .collect();

            if ready_routes.is_empty() {
                // This should not happen if circular dependency detection worked correctly
                return Err(HttpDiffError::invalid_config(
                    "Failed to compute execution plan: possible circular dependency in filtered routes".to_string(),
                ));
            }

            // Add ready routes to current batch
            for route in &ready_routes {
                current_batch.add_route(route.clone());
                remaining_routes.remove(route);

                // Reduce in-degree for all dependents of this route within the required set
                for dependent in &self.graph.get_dependents(route) {
                    if required_routes.contains(dependent) {
                        if let Some(degree) = in_degree.get_mut(dependent) {
                            *degree -= 1;
                        }
                    }
                }
            }

            plan.add_batch(current_batch);
            batch_number += 1;
        }

        // Validate completeness against the required routes set
        plan.validate_completeness(&required_routes)?;

        Ok(plan)
    }

    /// Compute all routes required to execute the given routes (including transitive dependencies)
    pub fn compute_required_dependencies(&self, target_routes: &HashSet<String>) -> Result<HashSet<String>> {
        let mut required = HashSet::new();
        let mut visited = HashSet::new();

        for route in target_routes {
            if !self.graph.contains_route(route) {
                return Err(HttpDiffError::invalid_config(format!(
                    "Target route '{}' does not exist in dependency graph",
                    route
                )));
            }
            self.collect_dependencies_recursive(route, &mut required, &mut visited);
        }

        Ok(required)
    }

    /// Recursively collect all dependencies for a route
    fn collect_dependencies_recursive(&self, route: &str, required: &mut HashSet<String>, visited: &mut HashSet<String>) {
        if visited.contains(route) {
            return;
        }
        visited.insert(route.to_string());
        required.insert(route.to_string());

        for dependency in &self.graph.get_dependencies(route) {
            self.collect_dependencies_recursive(dependency, required, visited);
        }
    }

    /// Get routes that can be executed immediately (no dependencies)
    pub fn get_independent_routes(&self) -> Vec<String> {
        self.graph.get_root_routes()
    }

    /// Check if a route has any dependencies
    pub fn has_dependencies(&self, route_name: &str) -> bool {
        !self.graph.get_dependencies(route_name).is_empty()
    }

    /// Get all dependencies for a specific route
    pub fn get_route_dependencies(&self, route_name: &str) -> HashSet<String> {
        self.graph.get_dependencies(route_name)
    }

    /// Get all routes that depend on a specific route
    pub fn get_route_dependents(&self, route_name: &str) -> HashSet<String> {
        self.graph.get_dependents(route_name)
    }

    /// Validate that all referenced dependencies exist
    pub fn validate_dependency_integrity(&self) -> Result<()> {
        for route in &self.graph.routes {
            for dependency in &self.graph.get_dependencies(route) {
                if !self.graph.contains_route(dependency) {
                    return Err(HttpDiffError::invalid_config(format!(
                        "Route '{}' depends on non-existent route '{}'",
                        route, dependency
                    )));
                }
            }
        }

        // Check for circular dependencies
        self.graph.detect_circular_dependencies()?;

        Ok(())
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ExecutionStats {
        let plan = self.compute_execution_plan().unwrap_or_default();
        let independent_count = self.get_independent_routes().len();
        let dependent_count = self.graph.route_count() - independent_count;

        ExecutionStats {
            total_routes: self.graph.route_count(),
            independent_routes: independent_count,
            dependent_routes: dependent_count,
            execution_batches: plan.batch_count(),
            max_parallelism: plan
                .batches
                .iter()
                .map(|batch| batch.len())
                .max()
                .unwrap_or(0),
        }
    }

    /// Generate a graphviz DOT representation of the dependency graph for visualization
    pub fn to_graphviz_dot(&self) -> String {
        let mut dot = String::from("digraph DependencyGraph {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box, style=rounded];\n\n");

        // Add all nodes
        for route in &self.graph.routes {
            let color = if self.graph.get_dependencies(route).is_empty() {
                "lightgreen" // Independent routes
            } else {
                "lightblue" // Dependent routes
            };
            dot.push_str(&format!("  \"{}\" [fillcolor={}, style=\"filled,rounded\"];\n", route, color));
        }

        dot.push('\n');

        // Add all edges (dependencies)
        for route in &self.graph.routes {
            for dependency in &self.graph.get_dependencies(route) {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", dependency, route));
            }
        }

        dot.push_str("}\n");
        dot
    }

    /// Generate a simple text visualization of the dependency graph
    pub fn to_text_visualization(&self) -> String {
        let mut output = String::new();
        output.push_str("Dependency Graph Visualization\n");
        output.push_str("==============================\n\n");

        // Show independent routes first
        let independent_routes = self.graph.get_root_routes();
        if !independent_routes.is_empty() {
            output.push_str("Independent Routes (no dependencies):\n");
            for route in &independent_routes {
                output.push_str(&format!("  ✓ {}\n", route));
            }
            output.push('\n');
        }

        // Show dependency chains
        for route in &self.graph.routes {
            let deps = self.graph.get_dependencies(route);
            if !deps.is_empty() {
                output.push_str(&format!("Route: {}\n", route));
                output.push_str("  Depends on:\n");
                for dep in &deps {
                    output.push_str(&format!("    └─ {}\n", dep));
                }
                output.push('\n');
            }
        }

        // Show dependents
        output.push_str("Dependents (routes that depend on others):\n");
        for route in &self.graph.routes {
            let dependents = self.graph.get_dependents(route);
            if !dependents.is_empty() {
                output.push_str(&format!("  {} is required by:\n", route));
                for dependent in &dependents {
                    output.push_str(&format!("    └─ {}\n", dependent));
                }
            }
        }

        output
    }

    /// Generate a summary report of the dependency graph structure
    pub fn generate_analysis_report(&self) -> String {
        let mut report = String::new();
        report.push_str("Dependency Graph Analysis Report\n");
        report.push_str("================================\n\n");

        // Basic statistics
        let total_routes = self.graph.route_count();
        let independent_routes = self.graph.get_root_routes();
        let dependent_count = total_routes - independent_routes.len();

        report.push_str(&format!("Total Routes: {}\n", total_routes));
        report.push_str(&format!("Independent Routes: {} ({:.1}%)\n", 
            independent_routes.len(), 
            (independent_routes.len() as f64 / total_routes as f64) * 100.0));
        report.push_str(&format!("Dependent Routes: {} ({:.1}%)\n", 
            dependent_count,
            (dependent_count as f64 / total_routes as f64) * 100.0));

        // Complexity analysis
        let max_dependencies = self.graph.routes.iter()
            .map(|route| self.graph.get_dependencies(route).len())
            .max()
            .unwrap_or(0);
        
        let avg_dependencies = if total_routes > 0 {
            self.graph.routes.iter()
                .map(|route| self.graph.get_dependencies(route).len())
                .sum::<usize>() as f64 / total_routes as f64
        } else {
            0.0
        };

        report.push_str(&format!("Maximum Dependencies per Route: {}\n", max_dependencies));
        report.push_str(&format!("Average Dependencies per Route: {:.2}\n", avg_dependencies));

        // Identify potential bottlenecks (routes with many dependents)
        let mut bottlenecks: Vec<_> = self.graph.routes.iter()
            .map(|route| (route, self.graph.get_dependents(route).len()))
            .filter(|(_, dependent_count)| *dependent_count > 1)
            .collect();
        bottlenecks.sort_by(|a, b| b.1.cmp(&a.1));

        if !bottlenecks.is_empty() {
            report.push_str("\nPotential Bottlenecks (routes with multiple dependents):\n");
            for (route, dependent_count) in &bottlenecks {
                report.push_str(&format!("  {} → {} dependents\n", route, dependent_count));
            }
        }

        // Identify long dependency chains
        let max_chain_length = self.find_longest_dependency_chain();
        report.push_str(&format!("\nLongest Dependency Chain: {} levels\n", max_chain_length));

        if max_chain_length > 3 {
            report.push_str("  ⚠️  Consider breaking long dependency chains for better parallelism\n");
        }

        report.push('\n');
        report
    }

    /// Find the length of the longest dependency chain in the graph
    fn find_longest_dependency_chain(&self) -> usize {
        let mut max_depth = 0;
        
        for route in &self.graph.routes {
            let depth = self.calculate_dependency_depth(route, &mut HashSet::new());
            max_depth = max_depth.max(depth);
        }
        
        max_depth
    }

    /// Calculate the maximum dependency depth for a route (recursive)
    fn calculate_dependency_depth(&self, route: &str, visited: &mut HashSet<String>) -> usize {
        if visited.contains(route) {
            return 0; // Avoid infinite recursion in case of cycles
        }
        
        visited.insert(route.to_string());
        
        let dependencies = self.graph.get_dependencies(route);
        if dependencies.is_empty() {
            visited.remove(route);
            return 1;
        }
        
        let max_dep_depth = dependencies.iter()
            .map(|dep| self.calculate_dependency_depth(dep, visited))
            .max()
            .unwrap_or(0);
        
        visited.remove(route);
        max_dep_depth + 1
    }
}

/// Represents dynamic dependency information for runtime resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicDependency {
    /// The route that has this dependency
    pub dependent_route: String,
    /// The route it depends on
    pub dependency_route: String,
    /// Whether to wait for value extraction completion
    pub wait_for_extraction: bool,
    /// Expected extracted value keys (empty if not specified)
    pub expected_values: Vec<String>,
}

impl DynamicDependency {
    /// Create a new dynamic dependency
    pub fn new(dependent_route: String, dependency_route: String) -> Self {
        Self {
            dependent_route,
            dependency_route,
            wait_for_extraction: false,
            expected_values: Vec::new(),
        }
    }

    /// Set whether to wait for value extraction
    pub fn with_extraction_wait(mut self, wait: bool) -> Self {
        self.wait_for_extraction = wait;
        self
    }

    /// Add expected extracted value keys
    pub fn with_expected_values(mut self, values: Vec<String>) -> Self {
        self.expected_values = values;
        self
    }
}

/// Tracks the dynamic state of route execution for value extraction dependencies
#[derive(Debug, Clone)]
pub struct DynamicExecutionState {
    /// Routes that have completed execution
    completed_routes: HashSet<String>,
    /// Routes that have completed value extraction
    extraction_completed: HashSet<String>,
    /// Dynamic dependencies with extraction requirements
    dynamic_dependencies: HashMap<String, Vec<DynamicDependency>>,
    /// Cache of computed ready routes to avoid recomputation
    ready_routes_cache: Option<Vec<String>>,
}

impl DynamicExecutionState {
    /// Create a new dynamic execution state
    pub fn new() -> Self {
        Self {
            completed_routes: HashSet::new(),
            extraction_completed: HashSet::new(),
            dynamic_dependencies: HashMap::new(),
            ready_routes_cache: None,
        }
    }

    /// Add a dynamic dependency
    pub fn add_dynamic_dependency(&mut self, dependency: DynamicDependency) {
        self.dynamic_dependencies
            .entry(dependency.dependent_route.clone())
            .or_insert_with(Vec::new)
            .push(dependency);
        self.invalidate_cache();
    }

    /// Mark a route as completed
    pub fn mark_route_completed(&mut self, route_name: &str) {
        self.completed_routes.insert(route_name.to_string());
        self.invalidate_cache();
    }

    /// Mark value extraction as completed for a route
    pub fn mark_extraction_completed(&mut self, route_name: &str) {
        self.extraction_completed.insert(route_name.to_string());
        self.invalidate_cache();
    }

    /// Check if a route can be executed based on dynamic dependencies
    pub fn can_execute_route(&self, route_name: &str, resolver: &DependencyResolver) -> bool {
        // First check static dependencies
        let static_dependencies = resolver.get_route_dependencies(route_name);
        for static_dep in &static_dependencies {
            if !self.completed_routes.contains(static_dep) {
                return false;
            }
        }

        // Then check dynamic dependencies if they exist
        if let Some(dynamic_deps) = self.dynamic_dependencies.get(route_name) {
            for dynamic_dep in dynamic_deps {
                if !self.completed_routes.contains(&dynamic_dep.dependency_route) {
                    return false;
                }

                if dynamic_dep.wait_for_extraction && !self.extraction_completed.contains(&dynamic_dep.dependency_route) {
                    return false;
                }
            }
        }

        true
    }

    /// Get routes that are ready to execute
    pub fn get_ready_routes(&mut self, resolver: &DependencyResolver) -> Vec<String> {
        if let Some(ref cache) = self.ready_routes_cache {
            return cache.clone();
        }

        let ready: Vec<String> = resolver
            .graph()
            .get_all_routes()
            .iter()
            .filter(|route| {
                !self.completed_routes.contains(*route) && self.can_execute_route(route, resolver)
            })
            .cloned()
            .collect();

        self.ready_routes_cache = Some(ready.clone());
        ready
    }

    /// Get completion statistics
    pub fn get_completion_stats(&self, total_routes: usize) -> DynamicExecutionStats {
        DynamicExecutionStats {
            total_routes,
            completed_routes: self.completed_routes.len(),
            remaining_routes: total_routes - self.completed_routes.len(),
            extraction_completed: self.extraction_completed.len(),
            has_dynamic_dependencies: !self.dynamic_dependencies.is_empty(),
        }
    }

    /// Reset the state for a new execution
    pub fn reset(&mut self) {
        self.completed_routes.clear();
        self.extraction_completed.clear();
        self.invalidate_cache();
    }

    /// Invalidate the ready routes cache
    fn invalidate_cache(&mut self) {
        self.ready_routes_cache = None;
    }
}

impl Default for DynamicExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about dynamic execution state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicExecutionStats {
    /// Total number of routes
    pub total_routes: usize,
    /// Number of completed routes
    pub completed_routes: usize,
    /// Number of remaining routes
    pub remaining_routes: usize,
    /// Number of routes with completed extraction
    pub extraction_completed: usize,
    /// Whether there are any dynamic dependencies
    pub has_dynamic_dependencies: bool,
}

impl DynamicExecutionStats {
    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        self.remaining_routes == 0
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_routes == 0 {
            100.0
        } else {
            (self.completed_routes as f64 / self.total_routes as f64) * 100.0
        }
    }

    /// Check if all routes have completed extraction
    pub fn all_extractions_complete(&self) -> bool {
        self.extraction_completed == self.completed_routes
    }
}

/// Statistics about dependency resolution and execution planning
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStats {
    /// Total number of routes
    pub total_routes: usize,
    /// Number of routes with no dependencies
    pub independent_routes: usize,
    /// Number of routes with dependencies
    pub dependent_routes: usize,
    /// Number of execution batches required
    pub execution_batches: usize,
    /// Maximum number of routes that can run in parallel (largest batch size)
    pub max_parallelism: usize,
}

impl ExecutionStats {
    /// Check if all routes are independent (can run in parallel)
    pub fn all_independent(&self) -> bool {
        self.dependent_routes == 0 && self.execution_batches <= 1
    }

    /// Check if there are any dependencies
    pub fn has_dependencies(&self) -> bool {
        self.dependent_routes > 0
    }

    /// Get the average batch size
    pub fn average_batch_size(&self) -> f64 {
        if self.execution_batches == 0 {
            0.0
        } else {
            self.total_routes as f64 / self.execution_batches as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_route(name: &str, depends_on: Option<Vec<String>>) -> Route {
        Route {
            name: name.to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            headers: None,
            params: None,
            base_urls: None,
            body: None,
            conditions: None,
            extract: None,
            depends_on,
            wait_for_extraction: None,
        }
    }

    #[test]
    fn test_dependency_graph_creation() {
        let mut graph = DependencyGraph::new();
        graph.add_route("route1".to_string());
        graph.add_route("route2".to_string());

        assert_eq!(graph.route_count(), 2);
        assert!(graph.contains_route("route1"));
        assert!(graph.contains_route("route2"));
    }

    #[test]
    fn test_dependency_graph_add_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_route("route1".to_string());
        graph.add_route("route2".to_string());

        // route2 depends on route1
        graph.add_dependency("route2".to_string(), "route1".to_string()).unwrap();

        assert_eq!(graph.get_dependencies("route2").len(), 1);
        assert!(graph.get_dependencies("route2").contains("route1"));
        assert_eq!(graph.get_dependents("route1").len(), 1);
        assert!(graph.get_dependents("route1").contains("route2"));
    }

    #[test]
    fn test_dependency_graph_root_routes() {
        let mut graph = DependencyGraph::new();
        graph.add_route("route1".to_string());
        graph.add_route("route2".to_string());
        graph.add_route("route3".to_string());

        // route2 depends on route1, route3 depends on route2
        graph.add_dependency("route2".to_string(), "route1".to_string()).unwrap();
        graph.add_dependency("route3".to_string(), "route2".to_string()).unwrap();

        let roots = graph.get_root_routes();
        assert_eq!(roots.len(), 1);
        assert!(roots.contains(&"route1".to_string()));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_route("route1".to_string());
        graph.add_route("route2".to_string());
        graph.add_route("route3".to_string());

        // Create a cycle: route1 -> route2 -> route3 -> route1
        graph.add_dependency("route2".to_string(), "route1".to_string()).unwrap();
        graph.add_dependency("route3".to_string(), "route2".to_string()).unwrap();
        graph.add_dependency("route1".to_string(), "route3".to_string()).unwrap();

        let result = graph.detect_circular_dependencies();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency detected"));
    }

    #[test]
    fn test_dependency_resolver_from_routes_simple() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();
        
        assert_eq!(resolver.graph().route_count(), 2);
        assert!(!resolver.has_dependencies("auth"));
        assert!(resolver.has_dependencies("users"));
    }

    #[test]
    fn test_dependency_resolver_invalid_dependency() {
        let routes = vec![
            create_test_route("users", Some(vec!["nonexistent".to_string()])),
        ];

        let result = DependencyResolver::from_routes(&routes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("depends on non-existent route"));
    }

    #[test]
    fn test_execution_plan_simple_chain() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
            create_test_route("profile", Some(vec!["users".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();
        let plan = resolver.compute_execution_plan().unwrap();

        assert_eq!(plan.batch_count(), 3);
        assert_eq!(plan.batches[0].routes, vec!["auth"]);
        assert_eq!(plan.batches[1].routes, vec!["users"]);
        assert_eq!(plan.batches[2].routes, vec!["profile"]);
    }

    #[test]
    fn test_execution_plan_parallel_routes() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
            create_test_route("posts", Some(vec!["auth".to_string()])),
            create_test_route("comments", Some(vec!["auth".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();
        let plan = resolver.compute_execution_plan().unwrap();

        assert_eq!(plan.batch_count(), 2);
        assert_eq!(plan.batches[0].routes, vec!["auth"]);
        // Second batch should contain users, posts, and comments in some order
        assert_eq!(plan.batches[1].routes.len(), 3);
        assert!(plan.batches[1].routes.contains(&"users".to_string()));
        assert!(plan.batches[1].routes.contains(&"posts".to_string()));
        assert!(plan.batches[1].routes.contains(&"comments".to_string()));
    }

    #[test]
    fn test_execution_plan_complex_dependencies() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("profile", None),
            create_test_route("users", Some(vec!["auth".to_string(), "profile".to_string()])),
            create_test_route("posts", Some(vec!["users".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();
        let plan = resolver.compute_execution_plan().unwrap();

        assert_eq!(plan.batch_count(), 3);
        
        // First batch: auth and profile (both independent)
        assert_eq!(plan.batches[0].routes.len(), 2);
        assert!(plan.batches[0].routes.contains(&"auth".to_string()));
        assert!(plan.batches[0].routes.contains(&"profile".to_string()));
        
        // Second batch: users (depends on both auth and profile)
        assert_eq!(plan.batches[1].routes, vec!["users"]);
        
        // Third batch: posts (depends on users)
        assert_eq!(plan.batches[2].routes, vec!["posts"]);
    }

    #[test]
    fn test_execution_stats() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("profile", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();
        let stats = resolver.get_execution_stats();

        assert_eq!(stats.total_routes, 3);
        assert_eq!(stats.independent_routes, 2); // auth and profile
        assert_eq!(stats.dependent_routes, 1);   // users
        assert_eq!(stats.execution_batches, 2);
        assert_eq!(stats.max_parallelism, 2);    // First batch has 2 routes
        assert!(!stats.all_independent());
        assert!(stats.has_dependencies());
    }

    #[test]
    fn test_execution_batch() {
        let mut batch = ExecutionBatch::new(0);
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);

        batch.add_route("route1".to_string());
        batch.add_route("route2".to_string());

        assert!(!batch.is_empty());
        assert_eq!(batch.len(), 2);
        assert_eq!(batch.batch_number, 0);
        assert_eq!(batch.routes, vec!["route1", "route2"]);
    }

    #[test]
    fn test_execution_plan_validation() {
        let mut plan = ExecutionPlan::new();
        let mut batch = ExecutionBatch::new(0);
        batch.add_route("route1".to_string());
        plan.add_batch(batch);

        let expected_routes = vec!["route1".to_string()].into_iter().collect();
        assert!(plan.validate_completeness(&expected_routes).is_ok());

        let expected_routes_missing = vec!["route1".to_string(), "route2".to_string()].into_iter().collect();
        assert!(plan.validate_completeness(&expected_routes_missing).is_err());
    }

    #[test]
    fn test_dependency_resolver_filtered_execution() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
            create_test_route("posts", Some(vec!["auth".to_string()])),
            create_test_route("comments", Some(vec!["posts".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();

        // Test filtered execution for just comments (should include auth and posts as dependencies)
        let target_routes: HashSet<String> = vec!["comments".to_string()].into_iter().collect();
        let plan = resolver.compute_execution_plan_filtered(&target_routes).unwrap();

        assert_eq!(plan.batch_count(), 3);
        assert_eq!(plan.total_routes, 3); // auth, posts, comments
        
        // First batch: auth
        assert_eq!(plan.batches[0].routes.len(), 1);
        assert!(plan.batches[0].routes.contains(&"auth".to_string()));
        
        // Second batch: posts
        assert_eq!(plan.batches[1].routes.len(), 1);
        assert!(plan.batches[1].routes.contains(&"posts".to_string()));
        
        // Third batch: comments
        assert_eq!(plan.batches[2].routes.len(), 1);
        assert!(plan.batches[2].routes.contains(&"comments".to_string()));
    }

    #[test]
    fn test_dependency_resolver_required_dependencies() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
            create_test_route("posts", Some(vec!["auth".to_string()])),
            create_test_route("comments", Some(vec!["posts".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();

        // Test that comments requires auth, posts, and comments
        let target_routes: HashSet<String> = vec!["comments".to_string()].into_iter().collect();
        let required = resolver.compute_required_dependencies(&target_routes).unwrap();

        assert_eq!(required.len(), 3);
        assert!(required.contains("auth"));
        assert!(required.contains("posts"));
        assert!(required.contains("comments"));

        // Test that users requires auth and users
        let target_routes: HashSet<String> = vec!["users".to_string()].into_iter().collect();
        let required = resolver.compute_required_dependencies(&target_routes).unwrap();

        assert_eq!(required.len(), 2);
        assert!(required.contains("auth"));
        assert!(required.contains("users"));
    }

    #[test]
    fn test_dynamic_execution_state() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
            create_test_route("posts", Some(vec!["users".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();
        let mut state = DynamicExecutionState::new();

        // Add dynamic dependency for posts to wait for users extraction
        let dynamic_dep = DynamicDependency::new("posts".to_string(), "users".to_string())
            .with_extraction_wait(true);
        state.add_dynamic_dependency(dynamic_dep);

        // Initially, only auth should be ready
        let ready = state.get_ready_routes(&resolver);
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&"auth".to_string()));

        // After auth completes, users should be ready
        state.mark_route_completed("auth");
        let ready = state.get_ready_routes(&resolver);
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&"users".to_string()));

        // After users completes but extraction is not done, posts should not be ready
        state.mark_route_completed("users");
        let ready = state.get_ready_routes(&resolver);
        assert!(ready.is_empty());

        // After users extraction completes, posts should be ready
        state.mark_extraction_completed("users");
        let ready = state.get_ready_routes(&resolver);
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&"posts".to_string()));
    }

    #[test]
    fn test_dependency_graph_performance_caching() {
        let mut graph = DependencyGraph::new();
        graph.add_route("route1".to_string());
        graph.add_route("route2".to_string());
        graph.add_route("route3".to_string());

        // First call should compute and cache
        let roots1 = graph.get_root_routes();
        assert_eq!(roots1.len(), 3);

        // Second call should use cache
        let roots2 = graph.get_root_routes();
        assert_eq!(roots1, roots2);

        // Adding a dependency should invalidate cache
        graph.add_dependency("route2".to_string(), "route1".to_string()).unwrap();
        let roots3 = graph.get_root_routes();
        assert_eq!(roots3.len(), 2); // Only route1 and route3 are roots now
        assert!(roots3.contains(&"route1".to_string()));
        assert!(roots3.contains(&"route3".to_string()));
    }

    #[test]
    fn test_dependency_graph_topological_ordering() {
        let mut graph = DependencyGraph::new();
        graph.add_route("auth".to_string());
        graph.add_route("users".to_string());
        graph.add_route("posts".to_string());

        graph.add_dependency("users".to_string(), "auth".to_string()).unwrap();
        graph.add_dependency("posts".to_string(), "users".to_string()).unwrap();

        let topo_order = graph.get_topological_order().unwrap();
        assert_eq!(topo_order.len(), 3);

        // auth should come before users, users should come before posts
        let auth_pos = topo_order.iter().position(|r| r == "auth").unwrap();
        let users_pos = topo_order.iter().position(|r| r == "users").unwrap();
        let posts_pos = topo_order.iter().position(|r| r == "posts").unwrap();

        assert!(auth_pos < users_pos);
        assert!(users_pos < posts_pos);

        // Second call should use cache
        let topo_order2 = graph.get_topological_order().unwrap();
        assert_eq!(topo_order, topo_order2);
    }

    #[test]
    fn test_dependency_graph_batch_update() {
        let mut graph = DependencyGraph::new();

        // Batch update should be more efficient for large operations
        let result = graph.batch_update(|g| {
            g.add_route("route1".to_string());
            g.add_route("route2".to_string());
            g.add_route("route3".to_string());
            g.add_dependency("route2".to_string(), "route1".to_string())?;
            g.add_dependency("route3".to_string(), "route2".to_string())?;
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(graph.route_count(), 3);
        
        let topo_order = graph.get_topological_order().unwrap();
        assert_eq!(topo_order, vec!["route1", "route2", "route3"]);
    }

    #[test]
    fn test_dependency_resolver_visualization() {
        let routes = vec![
            create_test_route("auth", None),
            create_test_route("users", Some(vec!["auth".to_string()])),
            create_test_route("posts", Some(vec!["auth".to_string()])),
        ];

        let resolver = DependencyResolver::from_routes(&routes).unwrap();

        // Test text visualization
        let text_viz = resolver.to_text_visualization();
        assert!(text_viz.contains("Independent Routes"));
        assert!(text_viz.contains("auth"));
        assert!(text_viz.contains("Depends on"));

        // Test graphviz DOT generation
        let dot = resolver.to_graphviz_dot();
        assert!(dot.contains("digraph DependencyGraph"));
        assert!(dot.contains("auth"));
        assert!(dot.contains("users"));
        assert!(dot.contains("posts"));
        assert!(dot.contains("->"));

        // Test analysis report
        let report = resolver.generate_analysis_report();
        assert!(report.contains("Dependency Graph Analysis Report"));
        assert!(report.contains("Total Routes: 3"));
        assert!(report.contains("Independent Routes: 1"));
        assert!(report.contains("Dependent Routes: 2"));
    }

    #[test]
    fn test_dynamic_execution_stats() {
        let mut state = DynamicExecutionState::new();
        state.mark_route_completed("route1");
        state.mark_route_completed("route2");
        state.mark_extraction_completed("route1");

        let stats = state.get_completion_stats(5);
        assert_eq!(stats.total_routes, 5);
        assert_eq!(stats.completed_routes, 2);
        assert_eq!(stats.remaining_routes, 3);
        assert_eq!(stats.extraction_completed, 1);
        assert_eq!(stats.completion_percentage(), 40.0);
        assert!(!stats.is_complete());
        assert!(!stats.all_extractions_complete());
    }
}