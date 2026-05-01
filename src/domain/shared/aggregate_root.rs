use crate::domain::shared::domain_event::DomainEvent;
use crate::domain::shared::entity::Entity;

/// Aggregate Root - 도메인 집합 루트
///
/// 규칙:
/// - Aggregate 상태 변경 시 반드시 Domain Event 발생
/// - Event는 불변 (immutable)
/// - 외부에서는 Aggregate를 통째로 수정 불가 → Command를 통해 간접적 변경
pub trait AggregateRoot: Entity {
    /// 처리 대기 중인 Domain Event 목록
    fn pending_events(&self) -> &[Box<dyn DomainEvent>];

    /// 대기 중인 Event를 모두 소거 (커밋 후 호출)
    fn clear_pending_events(&mut self);

    /// 새 Domain Event를 대기열에 추가
    fn raise_event(&mut self, event: Box<dyn DomainEvent>);
}
