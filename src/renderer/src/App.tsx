function App(): React.JSX.Element {
  return (
    <div className="app-layout">
      <div className="sidebar">
        <div className="sidebar-header">
          <div className="quick-open-trigger">
            <span>Quick Open</span>
            <span className="shortcut">{'\u2318'}K</span>
          </div>
        </div>
      </div>
      <div className="content-area">
        <p>mdview-electron</p>
      </div>
    </div>
  )
}

export default App
