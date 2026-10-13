export type Seat = {
  eventId: string;
  seatId: string;
  state: "available" | "held";
};

export type SeatHold = {
  eventId: string;
  seatId: string;
  state: "held";
};

export type SeatStore = {
  findSeat(eventId: string, seatId: string): Promise<Seat | undefined>;
  saveHold(hold: SeatHold): Promise<void>;
};

export async function holdSeat(
  eventId: string,
  seatId: string,
  store: SeatStore,
): Promise<SeatHold> {
  const seat = await store.findSeat(eventId, seatId);
  if (!seat) {
    throw new Error(`Unknown seat: ${seatId}`);
  }
  if (seat.state !== "available") {
    throw new Error(`Seat is not available: ${seatId}`);
  }
  const hold: SeatHold = { eventId, seatId, state: "held" };
  await store.saveHold(hold);
  return hold;
}
