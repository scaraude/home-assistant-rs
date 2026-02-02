export const URI_FRONTEND = {
    FLOORPLAN: "/floorplan",
    SENSORS: "/sensors",
    SWITCH_DETAIL: (id: string) => `/switch/${id}`,
    ENERGY_DETAIL: (id: string) => `/energy/${id}`,
    PRESENCE_DETAIL: (id: string) => `/presence/${id}`,
    CONSOMMATIONS: "/consommations",
    COMMANDER: "/commander",
    LOGS: "/logs",
    SENSOR_DETAIL: (id: string) => `/sensor/${id}`,
};