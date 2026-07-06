export function StatusButtons({ onConfirm, onDismiss }: { onConfirm: () => void; onDismiss: () => void }) {
  return (
    <div className="status-actions">
      <button type="button" onClick={(e) => { e.stopPropagation(); onConfirm(); }}>
        确认
      </button>
      <button type="button" onClick={(e) => { e.stopPropagation(); onDismiss(); }}>
        误报
      </button>
    </div>
  );
}
