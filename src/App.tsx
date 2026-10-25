export default function App() {
  return (
    <div className="app">
      <aside className="rail">
        <span className="brand-mark" aria-hidden="true">
          <svg className="icon">
            <use href="/icons/lucide.svg#audio-lines" />
          </svg>
        </span>
        <nav aria-label="Основная навигация">
          <button type="button">Файлы</button>
          <button type="button">Транскрипция</button>
          <button type="button">Экспорт</button>
        </nav>
      </aside>
      <main className="app-main">
        <h1>Transaudio</h1>
      </main>
    </div>
  );
}
