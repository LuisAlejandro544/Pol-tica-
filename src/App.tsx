/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 *
 * Propósito del archivo:
 * Punto de entrada web mínimo ("Hello World") solicitado explícitamente para
 * que el entorno de compilación Vite/React compile y ejecute sin errores mientras
 * el juego móvil real (Rust + Lua 5.4 + mapa SVG de Latinoamérica) vive en /game/.
 */

export default function App() {
  return (
    <main className="min-h-screen flex items-center justify-center bg-slate-950 text-slate-100">
      <h1 className="text-3xl font-semibold tracking-tight">Hello World</h1>
    </main>
  );
}
