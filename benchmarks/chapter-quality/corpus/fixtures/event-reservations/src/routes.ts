export type Route = "userList" | "notFound";

export function resolveRoute(request: Request): Route {
  const url = new URL(request.url);
  if (request.method === "GET" && url.pathname === "/users") {
    return "userList";
  }
  return "notFound";
}
