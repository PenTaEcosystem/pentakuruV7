// Only a successful response replaces the last conversation.
export function createChatState() {
  return {
    question: '', answer: '', notice: '', pending: false,
    begin() { this.pending = true; this.notice = '' },
    accept(question, answer) { this.question = question; this.answer = answer; this.pending = false; this.notice = '' },
    reject(message) { this.pending = false; this.notice = message },
    text() {
      const conversation = this.answer ? `ANH\n${this.question}\n\nPENTAKURU\n${this.answer}` : 'PENTAKURU\nChọn chức năng bên dưới. Nhập câu hỏi hoặc chạm mic để nói.\n\nKéo lên/xuống để đọc câu trả lời.'
      return this.notice ? `${conversation}\n\nTHÔNG BÁO\n${this.notice}` : conversation
    }
  }
}
